/// Analyzer for the aggregate() method.

use syntax::ast::{ExprAssign, ExprCall, ExprPath};

use ast::{Aggregate, Expression, Identifier};
use error::{Error, SqlResult, res};
use super::{check_argument_count, check_field, path_expr_to_identifier, propose_similar_name};
use state::{SqlFields, aggregates_singleton};

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
