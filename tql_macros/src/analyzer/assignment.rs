/// Argument to assignment converter.

use syntax::ast::Expr_::ExprAssign;

use ast::{Assignment, Expression, RValue};
use error::{Error, SqlResult, res};
use plugin::number_literal;
use state::SqlFields;
use super::{check_field, check_field_type, path_expr_to_identifier};

/// Analyze the types of the `Assignment`s.
pub fn analyze_assignments_types(assignments: &[Assignment], table_name: &str, errors: &mut Vec<Error>) {
    for assignment in assignments {
        check_field_type(table_name, &RValue::Identifier(assignment.identifier.clone()), &assignment.value, errors);
    }
}

/// Convert an `Expression` to an `Assignment`.
pub fn argument_to_assignment(arg: &Expression, table_name: &str, table: &SqlFields) -> SqlResult<Assignment> {
    let mut errors = vec![];
    let mut assignment = Assignment {
        identifier: "".to_owned(),
        value: number_literal(0),
    };
    if let ExprAssign(ref expr1, ref expr2) = arg.node {
        assignment.value = expr2.clone();
        if let Some(identifier) = path_expr_to_identifier(expr1, &mut errors) {
            assignment.identifier = identifier;
            check_field(&assignment.identifier, expr1.span, table_name, table, &mut errors);
        }
    }
    else {
        errors.push(Error::new(
            "Expected assignment".to_owned(), // TODO: améliorer ce message.
            arg.span,
        ));
    }
    res(assignment, errors)
}
