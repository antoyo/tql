/// Analyzer for the insert() method.

use std::collections::HashSet;

use syntax::codemap::{Span, Spanned};

use ast::{Assignment, AssignementOperator};
use error::Error;
use state::{SqlTable, get_primary_key_field};
use types::Type;

/// Check that the method call contains all the fields from the `table` and that all assignments
/// does not use an operation (e.g. +=).
pub fn check_insert_arguments(assignments: &[Assignment], position: Span, table: &SqlTable, errors: &mut Vec<Error>) {
    let mut names = HashSet::new();
    let mut missing_fields: Vec<&str> = vec![];
    for assignment in assignments {
        names.insert(assignment.identifier.clone());
        let operator = &assignment.operator.node;
        if *operator != AssignementOperator::Equal {
            errors.push(Error::new(format!("expected = but got {}", *operator), assignment.operator.span));
        }
    }
    let primary_key = get_primary_key_field(&table);

    for field in table.fields.keys() {
        if !names.contains(field) && Some(field) != primary_key.as_ref() {
            if let Some(&Spanned { node: Type::Nullable(_), .. }) = table.fields.get(field) {
                // Do not err about missing nullable field.
            }
            else {
                missing_fields.push(&field);
            }
        }
    }

    if !missing_fields.is_empty() {
        let fields = "`".to_owned() + &missing_fields.join("`, `") + "`";
        errors.push(Error::new_with_code(format!("missing fields: {}", fields), position, "E0063"));
    }

    // TODO: vérifier que la clé primaire n’est pas dans les champs insérés?
}
