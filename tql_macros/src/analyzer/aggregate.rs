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

/// Analyzer for the aggregate() method.

use proc_macro2::Span;
use syn::{
    BinOp,
    Expr,
    ExprAssign,
    ExprBinary,
    ExprCall,
    ExprParen,
    ExprPath,
    ExprUnary,
    UnOp,
};
use syn::spanned::Spanned;

use ast::{
    Aggregate,
    AggregateFilter,
    AggregateFilterExpression,
    AggregateFilters,
    Expression,
    Identifier,
    WithSpan,
    first_token_span,
};
use error::{Error, Result, res};
use new_ident;
use state::{SqlTable, aggregates_singleton};
use super::{
    check_argument_count,
    check_field,
    path_expr_to_identifier,
    path_expr_to_string,
    propose_similar_name,
};
use super::filter::{binop_to_logical_operator, binop_to_relational_operator, is_logical_operator, is_relational_operator};

/// Convert an `Expression` to an `Aggregate`.
pub fn argument_to_aggregate(arg: &Expression, _table: &SqlTable) -> Result<Aggregate> {
    let mut errors = vec![];
    let mut aggregate = Aggregate::default();
    let aggregates = aggregates_singleton();

    let call = get_call_from_aggregate(arg, &mut aggregate, &mut errors);

    if let Expr::Call(ExprCall { ref func, ref args, .. }) = *call {
        if let Some(identifier) = path_expr_to_string(func, &mut errors) {
            if let Some(sql_function) = aggregates.get(&identifier) {
                aggregate.function = sql_function.clone();
            }
            else {
                let mut error = Error::new_with_code(
                    &format!("unresolved name `{}`", identifier),
                    // NOTE: we only want the position of the function name, not the parenthesis
                    // and the arguments, hence, we only fetch the position of the first token.
                    first_token_span(arg),
                    "E0425",
                );
                propose_similar_name(&identifier, aggregates.keys().map(String::as_ref), &mut error);
                errors.push(error);
            }
        }

        if check_argument_count(args, 1, arg.span(), &mut errors) {
            if let Expr::Path(ExprPath { ref path, .. }) = **args.first().expect("first argument").item() {
                let path_ident = path.segments.first().unwrap().into_item().ident;
                aggregate.field = Some(path_ident);

                if aggregate.result_name.is_none() {
                    let result_name = aggregate.field.expect("Aggregate identifier").to_string() + "_" +
                        &aggregate.function.to_lowercase();
                    let mut ident = new_ident(&result_name);
                    // NOTE: violate the hygiene by assigning a known context to this new
                    // identifier.
                    ident.span = path_ident.span;
                    aggregate.result_name = Some(ident);
                }
            }
            else {
                // TODO: allow expressions (field2 / field1).
            }
        }
    }
    else {
        errors.push(Error::new(
            "Expected function call", // TODO: improve this message.
            arg.span(),
        ));
    }

    res(aggregate, errors)
}

/// Convert an `Expression` to a group `Identifier`.
pub fn argument_to_group(arg: &Expression, table: &SqlTable) -> Result<Identifier> {
    let mut errors = vec![];
    let mut group = "".to_string();

    if let Some(identifier) = path_expr_to_identifier(arg, &mut errors) {
        check_field(&identifier, arg.span(), table, &mut errors);
        group = identifier.to_string();
    }

    res(group, errors)
}

/// Convert a Rust binary expression to an `AggregateFilterExpression` for an aggregate filter.
fn binary_expression_to_aggregate_filter_expression(expr1: &Expression, op: &BinOp, expr2: &Expression, aggregates: &[Aggregate], table: &SqlTable) -> Result<AggregateFilterExpression> {
    // TODO: accumulate the errors instead of stopping at the first one.
    let filter1 = expression_to_aggregate_filter_expression(expr1, aggregates, table)?;
    // TODO: return errors instead of dummy.
    let dummy = AggregateFilterExpression::NoFilters;

    let filter =
        if is_logical_operator(op) {
            let filter2 = expression_to_aggregate_filter_expression(expr2, aggregates, table)?;
            AggregateFilterExpression::Filters(AggregateFilters {
                operand1: Box::new(filter1),
                operator: binop_to_logical_operator(op),
                operand2: Box::new(filter2),
            })
        }
        else if is_relational_operator(op) {
            if let AggregateFilterExpression::FilterValue(filter1) = filter1 {
                AggregateFilterExpression::Filter(AggregateFilter {
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

/// Check that an aggregate field exists.
fn check_aggregate_field<'a>(identifier: &str, aggregates: &'a [Aggregate], position: Span, errors: &mut Vec<Error>) -> Option<&'a Aggregate> {
    let result = aggregates.iter().find(|aggr| aggr.result_name.as_ref().map(|name| name.as_ref()) == Some(identifier));
    if let None = result {
        errors.push(Error::new(
            &format!("no aggregate field named `{}` found", identifier), // TODO: improve this message.
            position
        ));
        // TODO: propose similar names.
    }
    result
}

/// Convert a Rust expression to an `AggregateFilterExpression` for an aggregate filter.
pub fn expression_to_aggregate_filter_expression(arg: &Expression, aggregates: &[Aggregate], table: &SqlTable) -> Result<AggregateFilterExpression> {
    let mut errors = vec![];

    let filter =
        match *arg {
            Expr::Binary(ExprBinary { ref op, ref left, ref right, .. }) => {
                binary_expression_to_aggregate_filter_expression(left, op, right, aggregates, table)?
            },
            Expr::Path(ExprPath { ref path, .. }) => {
                let segment_ident = path.segments.first().unwrap().into_item().ident;
                let identifier = segment_ident.to_string();
                let aggregate = check_aggregate_field(&identifier, aggregates, segment_ident.span, &mut errors);
                if let Some(aggregate) = aggregate {
                    // Transform the aggregate field name to its SQL representation.
                    AggregateFilterExpression::FilterValue(WithSpan {
                        node: aggregate.clone(),
                        span: arg.span(),
                    })
                }
                else {
                    AggregateFilterExpression::NoFilters
                }
            },
            Expr::Paren(ExprParen { ref expr, .. }) => {
                let filter = expression_to_aggregate_filter_expression(expr, aggregates, table)?;
                AggregateFilterExpression::ParenFilter(Box::new(filter))
            },
            Expr::Unary(ExprUnary { op: UnOp::Not(_), ref expr, .. }) => {
                let filter = expression_to_aggregate_filter_expression(expr, aggregates, table)?;
                AggregateFilterExpression::NegFilter(Box::new(filter))
            },
            _ => {
                errors.push(Error::new(
                    "Expected binary operation", // TODO: improve this message.
                    arg.span(),
                ));
                AggregateFilterExpression::NoFilters
            },
        };

    res(filter, errors)
}

/// Get the call expression from an `arg` expression.
fn get_call_from_aggregate<'a>(arg: &'a Expression, aggregate: &mut Aggregate, errors: &mut Vec<Error>) -> &'a Expression {
    // If the `arg` expression is an assignment, the call is on the right side.
    if let Expr::Assign(ExprAssign { ref left, ref right, .. }) = *arg {
        if let Some(identifier) = path_expr_to_identifier(left, errors) {
            aggregate.result_name = Some(identifier); // TODO
        }
        right
    }
    else {
        arg
    }
}
