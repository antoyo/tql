/// Analyzer for the insert() method.

use std::collections::HashSet;

use syn::Span;

use ast::{Assignment, AssignementOperator, WithSpan};
use error::Error;
use state::{BothTypes, SqlTable, get_primary_key_field};
use types::Type;

/// Check that the method call contains all the fields from the `table` and that all assignments
/// does not use an operation (e.g. +=).
pub fn check_insert_arguments(assignments: &[Assignment], position: Span, table: &SqlTable, errors: &mut Vec<Error>) {
    let mut fields = HashSet::new();
    let mut missing_fields: Vec<&str> = vec![];

    // Check the assignment operators.
    for assignment in assignments {
        fields.insert(assignment.identifier.clone().expect("Assignment identifier"));
        let operator = &assignment.operator.node;
        if *operator != AssignementOperator::Equal {
            errors.push(Error::new(&format!("expected = but got {}", *operator), assignment.operator.span));
        }
    }
    let primary_key = get_primary_key_field(&table);

    for field in table.fields.keys() {
        if !fields.contains(field) && Some(field) != primary_key {
            if let Some(&BothTypes { ty: WithSpan { node: Type::Nullable(_), .. }, ..}) = table.fields.get(field) {
                // Do not err about missing nullable field.
            }
            else {
                missing_fields.push(field.as_ref());
            }
        }
    }

    if !missing_fields.is_empty() {
        let fields = "`".to_string() + &missing_fields.join("`, `") + "`";
        errors.push(Error::new_with_code(&format!("missing fields: {}", fields), position, "E0063"));
    }

    // TODO: check if the primary key is not in the inserted field?
}
