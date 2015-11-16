/// Analyzer for the filter() method.

use syntax::ast::{BinOp_, Expr, Path, SpannedIdent};
use syntax::ast::Expr_::{ExprBinary, ExprMethodCall, ExprParen, ExprPath, ExprUnary};
use syntax::ast::UnOp;
use syntax::codemap::{Span, Spanned};
use syntax::ptr::P;

use ast::{self, Expression, Filter, FilterExpression, Filters, FilterValue, LogicalOperator, RelationalOperator};
use error::{SqlResult, Error, res};
use state::{SqlMethod, SqlMethodTypes, SqlTable, methods_singleton};
use super::{check_field, check_field_type, check_type, check_type_filter_value, propose_similar_name};
use types::Type;

/// Analyze the types of the `FilterExpression`.
pub fn analyze_filter_types(filter: &FilterExpression, table_name: &str, errors: &mut Vec<Error>) {
    // TODO: check that operators are used with the good types (perhaps not necessary because all
    // types may support all operators)?
    match *filter {
        FilterExpression::Filter(ref filter) => {
            check_field_type(table_name, &filter.operand1, &filter.operand2, errors);
        },
        FilterExpression::Filters(ref filters) => {
            analyze_filter_types(&*filters.operand1, table_name, errors);
            analyze_filter_types(&*filters.operand2, table_name, errors);
        },
        FilterExpression::NegFilter(ref filter) => {
            analyze_filter_types(filter, table_name, errors);
        },
        FilterExpression::NoFilters => (),
        FilterExpression::ParenFilter(ref filter) => {
            analyze_filter_types(filter, table_name, errors);
        },
        FilterExpression::FilterValue(ref filter_value) => {
            check_type_filter_value(&Type::Bool, filter_value, table_name, errors);
        },
    }
}

/// Convert a Rust binary expression to a `FilterExpression`.
fn binary_expression_to_filter_expression(expr1: &Expression, op: BinOp_, expr2: &Expression, table: &SqlTable) -> SqlResult<FilterExpression> {
    // TODO: accumulate the errors instead of stopping when the first one is encountered.
    let filter1 = try!(expression_to_filter_expression(expr1, table));
    // TODO: return errors instead of dummy.
    let dummy = FilterExpression::NoFilters;

    let filter =
        if is_logical_operator(op) {
            let filter2 = try!(expression_to_filter_expression(expr2, table));
            FilterExpression::Filters(Filters {
                operand1: Box::new(filter1),
                operator: binop_to_logical_operator(op),
                operand2: Box::new(filter2),
            })
        }
        else if is_relational_operator(op) {
            if let FilterExpression::FilterValue(filter1) = filter1 {
                FilterExpression::Filter(Filter {
                    operand1: filter1.node,
                    operator: binop_to_relational_operator(op),
                    operand2: expr2.clone(),
                })
            }
            else {
                dummy
            }
        }
        else {
            dummy
        };
    Ok(filter)
}

/// Convert a `BinOp_` to an SQL `LogicalOperator`.
pub fn binop_to_logical_operator(binop: BinOp_) -> LogicalOperator {
    match binop {
        BinOp_::BiAdd => unreachable!(),
        BinOp_::BiSub => unreachable!(),
        BinOp_::BiMul => unreachable!(),
        BinOp_::BiDiv => unreachable!(),
        BinOp_::BiRem => unreachable!(),
        BinOp_::BiAnd => LogicalOperator::And,
        BinOp_::BiOr => LogicalOperator::Or,
        BinOp_::BiBitXor => unreachable!(),
        BinOp_::BiBitAnd => unreachable!(),
        BinOp_::BiBitOr => unreachable!(),
        BinOp_::BiShl => unreachable!(),
        BinOp_::BiShr => unreachable!(),
        BinOp_::BiEq => unreachable!(),
        BinOp_::BiLt => unreachable!(),
        BinOp_::BiLe => unreachable!(),
        BinOp_::BiNe => unreachable!(),
        BinOp_::BiGe => unreachable!(),
        BinOp_::BiGt => unreachable!(),
    }
}

/// Convert a `BinOp_` to an SQL `RelationalOperator`.
pub fn binop_to_relational_operator(binop: BinOp_) -> RelationalOperator {
    match binop {
        BinOp_::BiAdd => unreachable!(),
        BinOp_::BiSub => unreachable!(),
        BinOp_::BiMul => unreachable!(),
        BinOp_::BiDiv => unreachable!(),
        BinOp_::BiRem => unreachable!(),
        BinOp_::BiAnd => unreachable!(),
        BinOp_::BiOr => unreachable!(),
        BinOp_::BiBitXor => unreachable!(),
        BinOp_::BiBitAnd => unreachable!(),
        BinOp_::BiBitOr => unreachable!(),
        BinOp_::BiShl => unreachable!(),
        BinOp_::BiShr => unreachable!(),
        BinOp_::BiEq => RelationalOperator::Equal,
        BinOp_::BiLt => RelationalOperator::LesserThan,
        BinOp_::BiLe => RelationalOperator::LesserThanEqual,
        BinOp_::BiNe => RelationalOperator::NotEqual,
        BinOp_::BiGe => RelationalOperator::GreaterThan,
        BinOp_::BiGt => RelationalOperator::GreaterThanEqual,
    }
}

/// Check the type of the arguments of the method.
fn check_method_arguments(arguments: &[Expression], argument_types: &[Type], errors: &mut Vec<Error>) {
    for (i, argument) in arguments.iter().enumerate() {
        check_type(&argument_types[i], argument, errors);
    }
}

/// Convert a Rust expression to a `FilterExpression`.
pub fn expression_to_filter_expression(arg: &P<Expr>, table: &SqlTable) -> SqlResult<FilterExpression> {
    let mut errors = vec![];

    let filter =
        match arg.node {
            ExprBinary(Spanned { node: op, .. }, ref expr1, ref expr2) => {
                try!(binary_expression_to_filter_expression(expr1, op, expr2, table))
            },
            ExprMethodCall(identifier, _, ref exprs) => {
                FilterExpression::FilterValue(Spanned {
                    node: method_call_expression_to_filter_expression(identifier, &exprs, table, &mut errors),
                    span: arg.span,
                })
            },
            ExprPath(None, ref path) => {
                let identifier = path.segments[0].identifier.to_string();
                check_field(&identifier, path.span, table, &mut errors);
                FilterExpression::FilterValue(Spanned {
                    node: FilterValue::Identifier(identifier),
                    span: arg.span,
                })
            },
            ExprParen(ref expr) => {
                let filter = try!(expression_to_filter_expression(expr, table));
                FilterExpression::ParenFilter(box filter)
            },
            ExprUnary(UnOp::UnNot, ref expr) => {
                let filter = try!(expression_to_filter_expression(expr, table));
                FilterExpression::NegFilter(box filter)
            },
            _ => {
                errors.push(Error::new(
                    "Expected binary operation".to_owned(), // TODO: improve this message.
                    arg.span,
                ));
                FilterExpression::NoFilters
            },
        };

    res(filter, errors)
}

/// Get an SQL method and arguments by type and name.
fn get_method<'a>(object_type: &'a Spanned<Type>, exprs: &[Expression], method_name: &str, identifier: SpannedIdent, errors: &mut Vec<Error>) -> Option<(&'a SqlMethodTypes, Vec<Expression>)> {
    let methods = methods_singleton();
    let type_methods =
        if let Type::Nullable(_) = object_type.node {
            methods.get(&Type::Nullable(box Type::Generic))
        }
        else {
            methods.get(&object_type.node)
        };
    match type_methods {
        Some(type_methods) => {
            match type_methods.get(method_name) {
                Some(sql_method) => {
                    let arguments: Vec<Expression> = exprs[1..].iter().map(Clone::clone).collect();
                    check_method_arguments(&arguments, &sql_method.argument_types, errors);
                    Some((sql_method, arguments))
                },
                None => {
                    unknown_method(identifier.span, &object_type.node, method_name, Some(type_methods), errors);
                    None
                },
            }
        },
        None => {
            unknown_method(identifier.span, &object_type.node, method_name, None, errors);
            None
        },
    }
}

/// Check if a `BinOp_` is a `LogicalOperator`.
pub fn is_logical_operator(binop: BinOp_) -> bool {
    match binop {
        BinOp_::BiAnd | BinOp_::BiOr => true,
        _ => false,
    }
}

/// Check if a `BinOp_` is a `RelationalOperator`.
pub fn is_relational_operator(binop: BinOp_) -> bool {
    match binop {
        BinOp_::BiEq | BinOp_::BiLt | BinOp_::BiLe | BinOp_::BiNe | BinOp_::BiGe | BinOp_::BiGt => true,
        _ => false,
    }
}

/// Convert a method call expression to a filter expression.
fn method_call_expression_to_filter_expression(identifier: SpannedIdent, exprs: &[Expression], table: &SqlTable, errors: &mut Vec<Error>) -> FilterValue {
    let method_name = identifier.node.name.to_string();
    let dummy = FilterValue::Identifier("".to_owned());
    match exprs[0].node {
        ExprPath(_, ref path) => {
            path_method_call_to_filter(path, identifier, &method_name, exprs, table, errors)
        },
        _ => {
            errors.push(Error::new(
                "expected identifier".to_owned(), // TODO: improve this message.
                exprs[0].span,
            ));
            dummy
        },
    }
}

/// Convert a method call where the object is an identifier to a filter expression.
fn path_method_call_to_filter(path: &Path, identifier: SpannedIdent, method_name: &str, exprs: &[Expression], table: &SqlTable, errors: &mut Vec<Error>) -> FilterValue {
    // TODO: return errors instead of dummy.
    let dummy = FilterValue::Identifier("".to_owned());
    let object_name = path.segments[0].identifier.name.to_string();
    match table.fields.get(&object_name) {
        Some(object_type) => {
            let type_method = get_method(object_type, exprs, method_name, identifier, errors);

            if let Some((&SqlMethodTypes { ref template, .. }, ref arguments)) = type_method {
                FilterValue::MethodCall(ast::MethodCall {
                    arguments: arguments.clone(),
                    method_name: method_name.to_owned(),
                    object_name: object_name,
                    template: template.clone(),
                })
            }
            else {
                // NOTE: An error is emitted in the get_method() function.
                dummy
            }
        },
        None => {
            check_field(&object_name, path.span, table, errors);
            dummy
        },
    }
}

/// Add an error to the vector `errors` about an unknown SQL method.
/// It suggests a similar name if there exist one.
fn unknown_method(position: Span, object_type: &Type, method_name: &str, type_methods: Option<&SqlMethod>, errors: &mut Vec<Error>) {
    errors.push(Error::new(
        format!("no method named `{}` found for type `{}`", method_name, object_type),
        position,
    ));
    if let Some(type_methods) = type_methods {
        propose_similar_name(method_name, type_methods.keys(), position, errors);
    }
}
