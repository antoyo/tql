/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

/// Analyzer for the filter() method.

use syn::{
    BinOp,
    Expr,
    ExprBinary,
    ExprMethodCall,
    ExprKind,
    ExprParen,
    ExprPath,
    ExprUnary,
    Ident,
    Path,
    Span,
    UnOp,
};
use syn::delimited::Delimited;
use syn::tokens::Comma;

use ast::{
    self,
    Expression,
    Filter,
    FilterExpression,
    Filters,
    FilterValue,
    LogicalOperator,
    RelationalOperator,
    WithSpan,
    expr_span,
};
use error::{Error, Result, res};
use state::{SqlMethod, SqlMethodTypes, SqlTable, methods_singleton};
use super::{
    check_field,
    check_field_type,
    check_type,
    check_type_filter_value,
    propose_similar_name,
};
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
fn binary_expression_to_filter_expression(expr1: &Expression, op: &BinOp, expr2: &Expression, table: &SqlTable) -> Result<FilterExpression> {
    // TODO: accumulate the errors instead of stopping when the first one is encountered.
    let filter1 = expression_to_filter_expression(expr1, table)?;
    // TODO: return errors instead of dummy.
    let dummy = FilterExpression::NoFilters;

    let filter =
        if is_logical_operator(op) {
            let filter2 = expression_to_filter_expression(expr2, table)?;
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

/// Convert a `BinOp` to an SQL `LogicalOperator`.
pub fn binop_to_logical_operator(binop: &BinOp) -> LogicalOperator {
    match *binop {
        BinOp::And(_) => LogicalOperator::And,
        BinOp::Or(_) => LogicalOperator::Or,
        BinOp::Add(_) | BinOp::AddEq(_) | BinOp::Sub(_) | BinOp::SubEq(_) | BinOp::Mul(_) | BinOp::MulEq(_) |
            BinOp::Div(_) | BinOp::DivEq(_) | BinOp::Rem(_) | BinOp::RemEq(_) | BinOp::BitXor(_) |
            BinOp::BitXorEq(_) | BinOp::BitAnd(_) | BinOp::BitAndEq(_) | BinOp::BitOr(_) | BinOp::BitOrEq(_) |
            BinOp::Shl(_) | BinOp::ShlEq(_) | BinOp::Shr(_) | BinOp::ShrEq(_) | BinOp::Eq(_) | BinOp::Lt(_) |
            BinOp::Le(_) | BinOp::Ne(_) | BinOp::Ge(_) | BinOp::Gt(_) =>
            unreachable!("binop_to_logical_operator"),
    }
}

/// Convert a `BinOp` to an SQL `RelationalOperator`.
pub fn binop_to_relational_operator(binop: &BinOp) -> RelationalOperator {
    match *binop {
        BinOp::Eq(_) => RelationalOperator::Equal,
        BinOp::Lt(_) => RelationalOperator::LesserThan,
        BinOp::Le(_) => RelationalOperator::LesserThanEqual,
        BinOp::Ne(_) => RelationalOperator::NotEqual,
        BinOp::Ge(_) => RelationalOperator::GreaterThan,
        BinOp::Gt(_) => RelationalOperator::GreaterThanEqual,
        BinOp::Add(_) | BinOp::AddEq(_) | BinOp::Sub(_) | BinOp::SubEq(_) | BinOp::Mul(_) | BinOp::MulEq(_) |
            BinOp::Div(_) | BinOp::DivEq(_) | BinOp::Rem(_) | BinOp::RemEq(_) | BinOp::And(_) | BinOp::Or(_) |
            BinOp::BitXor(_) | BinOp::BitXorEq(_) | BinOp::BitAnd(_) | BinOp::BitAndEq(_) | BinOp::BitOr(_) |
            BinOp::BitOrEq(_) | BinOp::Shl(_) | BinOp::ShlEq(_) | BinOp::Shr(_) | BinOp::ShrEq(_) =>
            unreachable!("binop_to_relational_operator"),
    }
}

/// Check the type of the arguments of the method.
fn check_method_arguments(arguments: &[Expression], argument_types: &[Type], errors: &mut Vec<Error>) {
    for (argument, argument_type) in arguments.iter().zip(argument_types) {
        check_type(argument_type, argument, errors)
    }
}

/// Convert a Rust expression to a `FilterExpression`.
pub fn expression_to_filter_expression(arg: &Expression, table: &SqlTable) -> Result<FilterExpression> {
    let mut errors = vec![];

    let filter =
        match arg.node {
            ExprKind::Binary(ExprBinary { ref op, ref left, ref right }) => {
                binary_expression_to_filter_expression(left, op, right, table)?
            },
            ExprKind::MethodCall(ExprMethodCall { method, ref expr, ref args, .. }) => {
                FilterExpression::FilterValue(WithSpan {
                    node: method_call_expression_to_filter_expression(method, expr, args, table, &mut errors),
                    span: expr_span(&arg),
                })
            },
            ExprKind::Path(ExprPath { ref path, .. }) => {
                let identifier = path.segments.first().unwrap().into_item().ident;
                check_field(&identifier, identifier.span, table, &mut errors);
                FilterExpression::FilterValue(WithSpan {
                    node: FilterValue::Identifier(identifier),
                    span: expr_span(&arg),
                })
            },
            ExprKind::Paren(ExprParen { ref expr, .. }) => {
                let filter = expression_to_filter_expression(expr, table)?;
                FilterExpression::ParenFilter(Box::new(filter))
            },
            ExprKind::Unary(ExprUnary { op: UnOp::Not(_), ref expr }) => {
                let filter = expression_to_filter_expression(expr, table)?;
                FilterExpression::NegFilter(Box::new(filter))
            },
            _ => {
                errors.push(Error::new(
                    "Expected binary operation", // TODO: improve this message.
                    expr_span(arg),
                ));
                FilterExpression::NoFilters
            },
        };

    res(filter, errors)
}

/// Get an SQL method and arguments by type and name.
fn get_method<'a>(object_type: &'a WithSpan<Type>, args: &Delimited<Expr, Comma>, method_name: &str, identifier: Ident, errors: &mut Vec<Error>) -> Option<(&'a SqlMethodTypes, Vec<Expression>)> {
    let methods = methods_singleton();
    let type_methods =
        if let Type::Nullable(_) = object_type.node {
            methods.get(&Type::Nullable(Box::new(Type::Generic)))
        }
        else {
            methods.get(&object_type.node)
        };
    match type_methods {
        Some(type_methods) => {
            match type_methods.get(method_name) {
                Some(sql_method) => {
                    let arguments: Vec<Expression> = args.iter()
                        .map(|element| element.item().clone())
                        .cloned()
                        .collect();
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

/// Check if a `BinOp` is a `LogicalOperator`.
pub fn is_logical_operator(binop: &BinOp) -> bool {
    match *binop {
        BinOp::And(_) | BinOp::Or(_) => true,
        _ => false,
    }
}

/// Check if a `BinOp` is a `RelationalOperator`.
pub fn is_relational_operator(binop: &BinOp) -> bool {
    match *binop {
        BinOp::Eq(_) | BinOp::Lt(_) | BinOp::Le(_) | BinOp::Ne(_) | BinOp::Ge(_) | BinOp::Gt(_) => true,
        _ => false,
    }
}

/// Convert a method call expression to a filter expression.
fn method_call_expression_to_filter_expression(identifier: Ident, expr: &Expression, args: &Delimited<Expr, Comma>,
    table: &SqlTable, errors: &mut Vec<Error>) -> FilterValue
{
    let method_name = identifier.to_string();
    let dummy = FilterValue::None;
    match expr.node {
        ExprKind::Path(ExprPath { ref path, .. }) => {
            path_method_call_to_filter(path, identifier, &method_name, args, table, errors)
        },
        _ => {
            errors.push(Error::new(
                "expected identifier", // TODO: improve this message.
                expr_span(expr),
            ));
            dummy
        },
    }
}

/// Convert a method call where the object is an identifier to a filter expression.
fn path_method_call_to_filter(path: &Path, identifier: Ident, method_name: &str, args: &Delimited<Expr, Comma>,
                              table: &SqlTable, errors: &mut Vec<Error>) -> FilterValue
{
    // TODO: return errors instead of dummy.
    let dummy = FilterValue::None;
    let object_name = path.segments.first().unwrap().into_item().ident;
    match table.fields.get(&object_name) {
        Some(object_type) => {
            let type_method = get_method(&object_type.ty, args, method_name, identifier, errors);

            if let Some((&SqlMethodTypes { ref template, .. }, ref arguments)) = type_method {
                FilterValue::MethodCall(ast::MethodCall {
                    arguments: arguments.clone(),
                    method_name: method_name.to_string(),
                    object_name,
                    template: template.clone(),
                })
            }
            else {
                // NOTE: An error is emitted in the get_method() function.
                dummy
            }
        },
        None => {
            check_field(&object_name, object_name.span, table, errors);
            dummy
        },
    }
}

/// Add an error to the vector `errors` about an unknown SQL method.
/// It suggests a similar name if there is one.
fn unknown_method(position: Span, object_type: &Type, method_name: &str, type_methods: Option<&SqlMethod>, errors: &mut Vec<Error>) {
    let mut error = Error::new(
        &format!("no method named `{}` found for type `{}`", method_name, object_type),
        position,
    );
    if let Some(type_methods) = type_methods {
        propose_similar_name(method_name, type_methods.keys().map(String::as_ref), &mut error);
    }
    errors.push(error);
}
