/*
 * Copyright (c) 2018 Boucher, Antoni <bouanto@zoho.com>
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

/// Analyzer for the insert() method.

use std::collections::HashSet;

use proc_macro2::Span;

use ast::{Assignment, AssignementOperator, WithSpan};
use error::Error;
use state::BothTypes;
use types::Type;

/// Check that the method call contains all the fields from the `table` and that all assignments
/// does not use an operation (e.g. +=).
pub fn check_insert_arguments(assignments: &[Assignment], position: Span, errors: &mut Vec<Error>) {
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

    // TODO: attempt to create the table struct to see the missing fields (what about nullable
    // fields? => maybe create a dummy struct with only the mandatory fields in the derive? (what
    // will happen when the optional fields are specified?)).
    /*for field in table.fields.keys() {
        if !fields.contains(field) && Some(field) != primary_key {
            if let Some(&BothTypes { ty: WithSpan { node: Type::Nullable(_), .. }, ..}) = table.fields.get(field) {
                // Do not err about missing nullable field.
            }
            else {
                missing_fields.push(field.as_ref());
            }
        }
    }*/

    if !missing_fields.is_empty() {
        let fields = "`".to_string() + &missing_fields.join("`, `") + "`";
        errors.push(Error::new_with_code(&format!("missing fields: {}", fields), position, "E0063"));
    }

    // TODO: check if the primary key is not in the inserted field?
}
