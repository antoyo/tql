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

/// Analyzer for the insert() method.

use std::collections::HashSet;

use syntax::codemap::{Span, Spanned};

use ast::{Assignment, AssignementOperator};
use error::SqlError;
use state::{SqlTable, get_primary_key_field};
use types::Type;

/// Check that the method call contains all the fields from the `table` and that all assignments
/// does not use an operation (e.g. +=).
pub fn check_insert_arguments(assignments: &[Assignment], position: Span, table: &SqlTable, errors: &mut Vec<SqlError>) {
    let mut fields = HashSet::new();
    let mut missing_fields: Vec<&str> = vec![];

    // Check the assignment operators.
    for assignment in assignments {
        fields.insert(assignment.identifier.clone());
        let operator = &assignment.operator.node;
        if *operator != AssignementOperator::Equal {
            errors.push(SqlError::new(&format!("expected = but got {}", *operator), assignment.operator.span));
        }
    }
    let primary_key = get_primary_key_field(&table);

    for field in table.fields.keys() {
        if !fields.contains(field) && Some(field) != primary_key.as_ref() {
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
        errors.push(SqlError::new_with_code(&format!("missing fields: {}", fields), position, "E0063"));
    }

    // TODO: check if the primary key is not in the inserted field?
}
