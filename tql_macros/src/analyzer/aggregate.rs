/// Analyzer for the aggregate() method.

use syntax::ast::{BinOp_, ExprAssign, ExprCall, ExprParen, ExprPath, ExprUnary};
use syntax::ast::Expr_::ExprBinary;
use syntax::ast::UnOp;
use syntax::codemap::{Span, Spanned};

use ast::{Aggregate, AggregateFilter, AggregateFilterExpression, AggregateFilters, AggregateFilterValue, Expression, Identifier};
use error::{Error, SqlResult, res};
use gen::ToSql;
use state::{SqlFields, aggregates_singleton};
use super::{check_argument_count, check_field, path_expr_to_identifier, propose_similar_name};
use super::filter::{binop_to_logical_operator, binop_to_relational_operator, is_logical_operator, is_relational_operator};

/// Convert an `Expression` to an `Aggregate`.
pub fn argument_to_aggregate(arg: &Expression, _table_name: &str, _table: &SqlFields) -> SqlResult<Aggregate> {
    let mut errors = vec![];
    let mut aggregate = Aggregate {
        field: "".to_owned(),
        function: "".to_owned(),
        result_name: "".to_owned(),
    };
    let aggregates = aggregates_singleton();

    let call =
        if let ExprAssign(ref left_value, ref right_value) = arg.node {
            if let Some(identifier) = path_expr_to_identifier(left_value, &mut errors) {
                aggregate.result_name = identifier;
            }
            right_value
        }
        else {
            arg
        };

    if let ExprCall(ref function, ref arguments) = call.node {
        if let Some(identifier) = path_expr_to_identifier(function, &mut errors) {
            if let Some(sql_function) = aggregates.get(&identifier) {
                aggregate.function = sql_function.clone();
            }
            else {
                errors.push(Error::new_with_code(
                    format!("unresolved name `{}`", identifier),
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
                // TODO: aussi permettre des expressions (field2 / field1).
            }
        }
    }
    else {
        errors.push(Error::new(
            "Expected function call".to_owned(), // TODO: améliorer ce message.
            arg.span,
        ));
    }

    res(aggregate, errors)
}

/// Convert an `Expression` to a group `Identifier`.
pub fn argument_to_group(arg: &Expression, table_name: &str, table: &SqlFields) -> SqlResult<Identifier> {
    let mut errors = vec![];
    let mut group = "".to_owned();

    if let Some(identifier) = path_expr_to_identifier(arg, &mut errors) {
        check_field(&identifier, arg.span, table_name, table, &mut errors);
        group = identifier;
    }

    res(group, errors)
}

/// Convert a Rust binary expression to a `AggregateFilterExpression` for an aggregate filter.
fn binary_expression_to_aggregate_filter_expression(expr1: &Expression, op: BinOp_, expr2: &Expression, aggregates: &[Aggregate], table_name: &str, table: &SqlFields) -> SqlResult<AggregateFilterExpression> {
    // TODO: accumuler les erreurs au lieu d’arrêter à la première.
    let filter1 = try!(expression_to_aggregate_filter_expression(expr1, aggregates, table_name, table));
    // TODO: retourner des erreurs à la place de dummy.
    let dummy = AggregateFilterExpression::NoFilters;

    let filter =
        if is_logical_operator(op) {
            let filter2 = try!(expression_to_aggregate_filter_expression(expr2, aggregates, table_name, table));
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
    let result = aggregates.iter().find(|aggr| aggr.result_name == identifier);
    if let None = result {
        errors.push(Error::new(
            format!("no aggregate field named `{}` found", identifier), // TODO: améliorer ce message.
            position
        ));
        // TODO: proposer des noms similaires.
    }
    result
}

/// Convert a Rust expression to a `AggregateFilterExpression` for an aggregate filter.
pub fn expression_to_aggregate_filter_expression(arg: &Expression, aggregates: &[Aggregate], table_name: &str, table: &SqlFields) -> SqlResult<AggregateFilterExpression> {
    let mut errors = vec![];

    let filter =
        match arg.node {
            ExprBinary(Spanned { node: op, .. }, ref expr1, ref expr2) => {
                try!(binary_expression_to_aggregate_filter_expression(expr1, op, expr2, aggregates, table_name, table))
            },
            ExprPath(None, ref path) => {
                let identifier = path.segments[0].identifier.to_string();
                let aggregate = check_aggregate_field(&identifier, aggregates, path.span, &mut errors);
                if let Some(aggregate) = aggregate {
                    // Transform the aggregate field name to its SQL representation.
                    AggregateFilterExpression::FilterValue(Spanned {
                        node: AggregateFilterValue::Sql(aggregate.to_sql()),
                        span: arg.span,
                    })
                }
                else {
                    AggregateFilterExpression::NoFilters
                }
            },
            ExprParen(ref expr) => {
                let filter = try!(expression_to_aggregate_filter_expression(expr, aggregates, table_name, table));
                AggregateFilterExpression::ParenFilter(box filter)
            },
            ExprUnary(UnOp::UnNot, ref expr) => {
                let filter = try!(expression_to_aggregate_filter_expression(expr, aggregates, table_name, table));
                AggregateFilterExpression::NegFilter(box filter)
            },
            _ => {
                errors.push(Error::new(
                    "Expected binary operation".to_owned(), // TODO: corriger ce message.
                    arg.span,
                ));
                AggregateFilterExpression::NoFilters
            },
        };

    res(filter, errors)
}
