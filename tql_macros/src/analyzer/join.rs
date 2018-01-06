/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
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
use error::{Result, res};
use state::{SqlTable, get_primary_key_field, tables_singleton};
use super::{check_field, mismatched_types, no_primary_key, path_expr_to_identifier};
use types::Type;

/// Convert an `Expression` to a `Join`
pub fn argument_to_join(arg: &Expression, table: &SqlTable) -> Result<Join> {
    let mut errors = vec![];
    let mut join = Join::default();

    if let Some(identifier) = path_expr_to_identifier(arg, &mut errors) {
        let name = identifier.to_string();
        check_field(&identifier, arg.span(), table, &mut errors);
        match table.fields.get(&identifier) {
            Some(types) => {
                let field_type = &types.ty.node;
                if let Type::Custom(ref related_table_name) = *field_type {
                    let sql_tables = tables_singleton();
                    if let Some(related_table) = sql_tables.get(related_table_name) {
                        match get_primary_key_field(related_table) {
                            Some(primary_key_field) =>
                                join = Join {
                                    base_field: name,
                                    base_table: table.name.to_string(),
                                    joined_field: primary_key_field.to_string(),
                                    joined_table: related_table_name.clone(),
                                },
                            None => errors.push(no_primary_key(related_table_name, related_table.position)),
                        }
                    }
                    // NOTE: if the field type is not an SQL table, an error is thrown by the
                    // linter.
                }
                else {
                    mismatched_types("ForeignKey<_>", field_type, arg.span(), &mut errors);
                }
            },
            None => (), // NOTE: This case is handled by the check_field() call above.
        }
    }
    res(join, errors)
}
