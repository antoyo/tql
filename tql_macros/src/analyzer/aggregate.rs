/*
 * Copyright (C) 2015  Boucher, Antoni <bouanto@zoho.com>
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

/// Analyzer for the aggregate() method.

use syntax::ast::{BinOp_, ExprAssign, ExprCall, ExprParen, ExprPath, ExprUnary};
use syntax::ast::Expr_::ExprBinary;
use syntax::ast::UnOp;
use syntax::codemap::{Span, Spanned};

use ast::{Aggregate, AggregateFilter, AggregateFilterExpression, AggregateFilters, Expression, Identifier};
use error::{SqlError, SqlResult, res};
use state::{SqlTable, aggregates_singleton};
use super::{check_argument_count, check_field, path_expr_to_identifier, propose_similar_name};
use super::filter::{binop_to_logical_operator, binop_to_relational_operator, is_logical_operator, is_relational_operator};

/// Convert an `Expression` to an `Aggregate`.
pub fn argument_to_aggregate(arg: &Expression, _table: &SqlTable) -> SqlResult<Aggregate> {
    let mut errors = vec![];
    let mut aggregate = Aggregate::default();
    let aggregates = aggregates_singleton();

    let call = get_call_from_aggregate(arg, &mut aggregate, &mut errors);

    if let ExprCall(ref function, ref arguments) = call.node {
        if let Some(identifier) = path_expr_to_identifier(function, &mut errors) {
            if let Some(sql_function) = aggregates.get(&identifier) {
                aggregate.function = sql_function.clone();
            }
            else {
                errors.push(SqlError::new_with_code(
                    &format!("unresolved name `{}`", identifier),
                    arg.span,
                    "E0425",
                ));
                propose_similar_name(&identifier, aggregates.keys(), arg.span, &mut errors);
            }
        }

        if check_argument_count(arguments, 1, arg.span, &mut errors) {
            if let ExprPath(_, ref path) = arguments[0].node {
                aggregate.field = path.segments[0].identifier.to_string();

                if aggregate.result_name.is_empty() {
                    aggregate.result_name = aggregate.field.clone() + "_" + &aggregate.function.to_lowercase();
                }
            }
            else {
                // TODO: allow expressions (field2 / field1).
            }
        }
    }
    else {
        errors.push(SqlError::new(
            "Expected function call", // TODO: improve this message.
            arg.span,
        ));
    }

    res(aggregate, errors)
}

/// Convert an `Expression` to a group `Identifier`.
pub fn argument_to_group(arg: &Expression, table: &SqlTable) -> SqlResult<Identifier> {
    let mut errors = vec![];
    let mut group = "".to_owned();

    if let Some(identifier) = path_expr_to_identifier(arg, &mut errors) {
        check_field(&identifier, arg.span, table, &mut errors);
        group = identifier;
    }

    res(group, errors)
}

/// Convert a Rust binary expression to an `AggregateFilterExpression` for an aggregate filter.
fn binary_expression_to_aggregate_filter_expression(expr1: &Expression, op: BinOp_, expr2: &Expression, aggregates: &[Aggregate], table: &SqlTable) -> SqlResult<AggregateFilterExpression> {
    // TODO: accumulate the errors instead of stopping at the first one.
    let filter1 = try!(expression_to_aggregate_filter_expression(expr1, aggregates, table));
    // TODO: return errors instead of dummy.
    let dummy = AggregateFilterExpression::NoFilters;

    let filter =
        if is_logical_operator(op) {
            let filter2 = try!(expression_to_aggregate_filter_expression(expr2, aggregates, table));
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
fn check_aggregate_field<'a>(identifier: &str, aggregates: &'a [Aggregate], position: Span, errors: &mut Vec<SqlError>) -> Option<&'a Aggregate> {
    let result = aggregates.iter().find(|aggr| aggr.result_name == identifier);
    if let None = result {
        errors.push(SqlError::new(
            &format!("no aggregate field named `{}` found", identifier), // TODO: improve this message.
            position
        ));
        // TODO: propose similar names.
    }
    result
}

/// Convert a Rust expression to an `AggregateFilterExpression` for an aggregate filter.
pub fn expression_to_aggregate_filter_expression(arg: &Expression, aggregates: &[Aggregate], table: &SqlTable) -> SqlResult<AggregateFilterExpression> {
    let mut errors = vec![];

    let filter =
        match arg.node {
            ExprBinary(Spanned { node: op, .. }, ref expr1, ref expr2) => {
                try!(binary_expression_to_aggregate_filter_expression(expr1, op, expr2, aggregates, table))
            },
            ExprPath(None, ref path) => {
                let identifier = path.segments[0].identifier.to_string();
                let aggregate = check_aggregate_field(&identifier, aggregates, path.span, &mut errors);
                if let Some(aggregate) = aggregate {
                    // Transform the aggregate field name to its SQL representation.
                    AggregateFilterExpression::FilterValue(Spanned {
                        node: aggregate.clone(),
                        span: arg.span,
                    })
                }
                else {
                    AggregateFilterExpression::NoFilters
                }
            },
            ExprParen(ref expr) => {
                let filter = try!(expression_to_aggregate_filter_expression(expr, aggregates, table));
                AggregateFilterExpression::ParenFilter(box filter)
            },
            ExprUnary(UnOp::UnNot, ref expr) => {
                let filter = try!(expression_to_aggregate_filter_expression(expr, aggregates, table));
                AggregateFilterExpression::NegFilter(box filter)
            },
            _ => {
                errors.push(SqlError::new(
                    "Expected binary operation", // TODO: improve this message.
                    arg.span,
                ));
                AggregateFilterExpression::NoFilters
            },
        };

    res(filter, errors)
}

/// Get the call expression from an `arg` expression.
fn get_call_from_aggregate<'a>(arg: &'a Expression, aggregate: &mut Aggregate, errors: &mut Vec<SqlError>) -> &'a Expression {
    // If the `arg` expression is an assignment, the call is on the right side.
    if let ExprAssign(ref left_value, ref right_value) = arg.node {
        if let Some(identifier) = path_expr_to_identifier(left_value, errors) {
            aggregate.result_name = identifier; // TODO
        }
        right_value
    }
    else {
        arg
    }
}
