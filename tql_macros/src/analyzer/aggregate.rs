/*
 * Copyright (c) 2017-2018 Boucher, Antoni <bouanto@zoho.com>
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
    ExprUnary,
    Ident,
    UnOp,
};
use syn::spanned::Spanned;

use ast::{
    Aggregate,
    AggregateFilter,
    AggregateFilterExpression,
    AggregateFilters,
    Expression,
    Query,
    WithSpan,
    first_token_span,
};
use error::{Error, Result, res};
use plugin::new_ident;
use state::aggregates_singleton;
use super::{
    check_argument_count,
    path_expr_to_identifier,
    path_expr_to_string,
    propose_similar_name,
};
use super::filter::{binop_to_logical_operator, binop_to_relational_operator, is_logical_operator, is_relational_operator};

/// Convert an `Expression` to an `Aggregate`.
pub fn argument_to_aggregate(arg: &Expression) -> Result<Aggregate> {
    let mut errors = vec![];
    let mut aggregate = Aggregate::default();
    let aggregates = aggregates_singleton();

    let call = get_call_from_aggregate(arg, &mut aggregate, &mut errors);

    if let Expr::Call(ref call) = *call {
        if let Some(identifier) = path_expr_to_string(&call.func, &mut errors) {
            aggregate.function = identifier.to_string();
            if let Some(sql_function) = aggregates.get(&identifier) {
                aggregate.sql_function = sql_function.clone();
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

        if check_argument_count(&call.args, 1, crate::merge_spans_of(arg), &mut errors) {
            if let Expr::Path(ref path) = **call.args.first().expect("first argument").value() {
                let path_ident = &path.path.segments.first().unwrap().into_value().ident;
                aggregate.field = Some(path_ident.clone());

                if aggregate.result_name.is_none() {
                    let result_name = aggregate.field.clone().expect("Aggregate identifier").to_string() + "_" +
                        &aggregate.sql_function.to_lowercase();
                    let mut ident = new_ident(&result_name);
                    // NOTE: violate the hygiene by assigning a known context to this new
                    // identifier.
                    ident.set_span(path_ident.span());
                    aggregate.result_name = Some(ident);
                }
                else {
                    aggregate.has_name_in_query = true;
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

/// Convert an `Expression` to a group `Ident`.
pub fn argument_to_group(arg: &Expression) -> Result<Ident> {
    let mut errors = vec![];
    let mut group = Ident::new("dummy_ident", Span::call_site());

    if let Some(identifier) = path_expr_to_identifier(arg, &mut errors) {
        group = identifier;
    }
    else {
        errors.push(Error::new(
            "Expected identifier", // TODO: improve this message.
            arg.span(),
        ));
    }

    res(group, errors)
}

/// Convert a Rust binary expression to an `AggregateFilterExpression` for an aggregate filter.
fn binary_expression_to_aggregate_filter_expression(expr1: &Expression, op: &BinOp, expr2: &Expression, aggregates: &[Aggregate]) -> Result<AggregateFilterExpression> {
    // TODO: accumulate the errors instead of stopping at the first one.
    let filter1 = expression_to_aggregate_filter_expression(expr1, aggregates)?;
    // TODO: return errors instead of dummy.
    let dummy = AggregateFilterExpression::NoFilters;

    let filter =
        if is_logical_operator(op) {
            let filter2 = expression_to_aggregate_filter_expression(expr2, aggregates)?;
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
    let matching = Some(identifier.to_string());
    let result = aggregates.iter().find(|aggr| aggr.result_name.as_ref().map(|name| name.to_string()) == matching);
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
pub fn expression_to_aggregate_filter_expression(arg: &Expression, aggregates: &[Aggregate]) -> Result<AggregateFilterExpression> {
    let mut errors = vec![];

    let filter =
        match *arg {
            Expr::Binary(ref bin) => {
                binary_expression_to_aggregate_filter_expression(&bin.left, &bin.op, &bin.right, aggregates)?
            },
            Expr::Path(ref path) => {
                let segment_ident = &path.path.segments.first().unwrap().into_value().ident;
                let identifier = segment_ident.to_string();
                let aggregate = check_aggregate_field(&identifier, aggregates, segment_ident.span(), &mut errors);
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
            Expr::Paren(ref paren) => {
                let filter = expression_to_aggregate_filter_expression(&paren.expr, aggregates)?;
                AggregateFilterExpression::ParenFilter(Box::new(filter))
            },
            Expr::Unary(ExprUnary { op: UnOp::Not(_), ref expr, .. }) => {
                let filter = expression_to_aggregate_filter_expression(expr, aggregates)?;
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
    if let Expr::Assign(ref assign) = *arg {
        if let Some(identifier) = path_expr_to_identifier(&assign.left, errors) {
            aggregate.result_name = Some(identifier); // TODO
        }
        &assign.right
    }
    else {
        arg
    }
}

/// Get the identifier in the group by clause to be able to check that they exist.
pub fn get_values_idents(query: &Query) -> Vec<Ident> {
    let mut idents = vec![];
    if let Query::Aggregate { ref groups, ..} = *query {
        for group in groups {
            idents.push(group.clone());
        }
    }
    idents
}

/// Get the aggregate function calls to typecheck the query.
pub fn get_aggregate_calls(query: &Query) -> Vec<(String, Expr)> {
    if let Query::Aggregate { ref aggregate_filter, ..} = *query {
        return get_calls_from_aggregate_filter(aggregate_filter);
    }
    vec![]
}

fn get_calls_from_aggregate_filter(filter: &AggregateFilterExpression) -> Vec<(String, Expr)> {
    let mut calls = vec![];
    match *filter {
        AggregateFilterExpression::Filter(ref filter) =>
            calls.push((filter.operand1.function.clone(), filter.operand2.clone())),
        AggregateFilterExpression::Filters(ref filters) => {
            calls.extend(get_calls_from_aggregate_filter(&filters.operand1));
            calls.extend(get_calls_from_aggregate_filter(&filters.operand2));
        },
        AggregateFilterExpression::FilterValue(_) => (),
        AggregateFilterExpression::NegFilter(ref filter) =>
            calls.extend(get_calls_from_aggregate_filter(filter)),
        AggregateFilterExpression::NoFilters => (),
        AggregateFilterExpression::ParenFilter(ref filter) =>
            calls.extend(get_calls_from_aggregate_filter(filter)),
    }
    calls
}
