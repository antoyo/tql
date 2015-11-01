/// Analyzer for the aggregate() method.

use syntax::ast::{ExprCall, ExprPath};

use ast::{Aggregate, Expression};
use error::{Error, SqlResult, res};
use super::propose_similar_name;
use state::{SqlFields, aggregates_singleton};

/// Convert an `Expression` to an `Aggregate`.
pub fn argument_to_aggregate(arg: &Expression, _table_name: &str, _table: &SqlFields) -> SqlResult<Aggregate> {
    let mut errors = vec![];
    let mut aggregate = Aggregate {
        field: "".to_owned(),
        function: "".to_owned(),
    };
    let aggregates = aggregates_singleton();

    if let ExprCall(ref function, ref arguments) = arg.node {
            if let ExprPath(_, ref path) = function.node {
                let identifier = path.segments[0].identifier.to_string();
                if let Some(sql_function) = aggregates.get(&identifier) {
                    aggregate.function = sql_function.clone();
                }
                else {
                    // TODO: afficher des noms similaires.
                    errors.push(Error::new_with_code(
                        format!("unresolved name `{}`", identifier),
                        arg.span,
                        "E0425",
                    ));
                    propose_similar_name(&identifier, aggregates.keys(), arg.span, &mut errors);
                }
            }
            else {
                // TODO
            }

            if arguments.len() == 1 {
                if let ExprPath(_, ref path) = arguments[0].node {
                    aggregate.field = path.segments[0].identifier.to_string();
                }
                else {
                    // TODO: aussi permettre des expressions (field2 / field1).
                }
            }
            else {
                errors.push(Error::new(
                    format!("expected one argument to the function but got {}", arguments.len()), // TODO: Améliorer ce message.
                    arg.span,
                ));
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
