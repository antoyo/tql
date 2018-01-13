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
use syn::Ident;

use ast::{
    Assignment,
    AssignementOperator,
    Query,
};
use error::Error;
use parser::MethodCalls;

/// Check that the method call contains all the fields from the `table` and that all assignments
/// does not use an operation (e.g. +=).
pub fn check_insert_arguments(assignments: &[Assignment], errors: &mut Vec<Error>) {
    let mut fields = HashSet::new();

    // Check the assignment operators.
    for assignment in assignments {
        fields.insert(assignment.identifier.clone().expect("Assignment identifier"));
        let operator = &assignment.operator.node;
        if *operator != AssignementOperator::Equal {
            errors.push(Error::new(&format!("expected = but got {}", *operator), assignment.operator.span));
        }
    }

    // TODO: check if the primary key is not in the inserted field?
}

pub fn get_insert_idents(query: &Query) -> Option<Vec<Ident>> {
    let mut idents = vec![];
    if let Query::Insert { ref assignments, ..} = *query {
        for assignment in assignments {
            if let Some(ref ident) = assignment.identifier {
                idents.push(ident.clone());
            }
        }
        idents.sort();
        return Some(idents)
    }
    None
}

pub fn get_insert_position(method_calls: &MethodCalls) -> Option<Span> {
    for call in &method_calls.calls {
        if call.name == "insert" {
            return Some(call.position);
        }
    }
    None
}
