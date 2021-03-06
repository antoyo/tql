/*
 * Copyright (c) 2017-2018 Boucher, Antoni <bouanto@zoho.com>
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

/// Analyzer for the join() method.

use syn::spanned::Spanned;

use ast::{Expression, Join};
use error::{Error, Result, res};
use super::path_expr_to_identifier;

/// Convert an `Expression` to a `Join`
pub fn argument_to_join(arg: &Expression, table_name: &str) -> Result<Join> {
    let mut errors = vec![];
    let join;

    if let Some(identifier) = path_expr_to_identifier(&arg, &mut errors) {
        join = Some(Join {
            base_field: identifier,
            base_table: table_name.to_string(),
        });
        // NOTE: if the field type is not an SQL table, an error is thrown.
    }
    else {
        return Err(vec![Error::new("Expecting identifier, but got", arg.span())]); // TODO: improve error message.
    }

    res(join.expect("join"), errors)
}
