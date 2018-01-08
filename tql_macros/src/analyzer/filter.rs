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

use proc_macro2::Span;
use syn::{
    BinOp,
    Expr,
    ExprUnary,
    Ident,
    Path,
    UnOp,
};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;

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
};
use error::{Error, Result, res};
use state::{SqlMethodTypes, methods_singleton};
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
fn binary_expression_to_filter_expression(expr1: &Expression, op: &BinOp, expr2: &Expression, table_name: &str) ->
    Result<FilterExpression>
{
    // TODO: accumulate the errors instead of stopping when the first one is encountered.
    let filter1 = expression_to_filter_expression(expr1, table_name)?;
    // TODO: return errors instead of dummy.
    let dummy = FilterExpression::NoFilters;

    let filter =
        if is_logical_operator(op) {
            let filter2 = expression_to_filter_expression(expr2, table_name)?;
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
pub fn expression_to_filter_expression(arg: &Expression, table_name: &str) -> Result<FilterExpression> {
    let mut errors = vec![];

    let filter =
        match *arg {
            Expr::Binary(ref bin) => {
                binary_expression_to_filter_expression(&bin.left, &bin.op, &bin.right, table_name)?
            },
            Expr::MethodCall(ref call) => {
                FilterExpression::FilterValue(WithSpan {
                    node: method_call_expression_to_filter_expression(call.method, &call.receiver, &call.args,
                                                                      &mut errors),
                    span: arg.span(),
                })
            },
            Expr::Path(ref path) => {
                let identifier = path.path.segments.first().unwrap().into_value().ident;
                check_field(&identifier, identifier.span, &mut errors);
                FilterExpression::FilterValue(WithSpan {
                    node: FilterValue::Identifier(table_name.to_string(), identifier),
                    span: arg.span(),
                })
            },
            Expr::Paren(ref paren) => {
                let filter = expression_to_filter_expression(&paren.expr, table_name)?;
                FilterExpression::ParenFilter(Box::new(filter))
            },
            Expr::Unary(ExprUnary { op: UnOp::Not(_), ref expr, .. }) => {
                let filter = expression_to_filter_expression(expr, table_name)?;
                FilterExpression::NegFilter(Box::new(filter))
            },
            _ => {
                errors.push(Error::new(
                    "Expected binary operation", // TODO: improve this message.
                    arg.span(),
                ));
                FilterExpression::NoFilters
            },
        };

    res(filter, errors)
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
fn method_call_expression_to_filter_expression(identifier: Ident, expr: &Expression, args: &Punctuated<Expr, Comma>,
    errors: &mut Vec<Error>) -> FilterValue
{
    let method_name = identifier.to_string();
    let dummy = FilterValue::None;
    match *expr {
        Expr::Path(ref path) => {
            path_method_call_to_filter(&path.path, identifier, &method_name, args, errors)
        },
        _ => {
            errors.push(Error::new(
                "expected identifier", // TODO: improve this message.
                expr.span(),
            ));
            dummy
        },
    }
}

/// Convert a method call where the object is an identifier to a filter expression.
fn path_method_call_to_filter(path: &Path, identifier: Ident, method_name: &str, args: &Punctuated<Expr, Comma>,
                              errors: &mut Vec<Error>) -> FilterValue
{
    // TODO: return errors instead of dummy.
    let dummy = FilterValue::None;
    // TODO: check the method call (types, arguments and if it exists).
    // TODO: check if the field exists.
    let object_name = path.segments.first().unwrap().into_value().ident;
    let arguments: Vec<Expression> = args.iter()
        .cloned()
        .collect();

    FilterValue::MethodCall(ast::MethodCall {
        arguments: arguments.clone(),
        method_name: method_name.to_string(),
        object_name,
    })
}
