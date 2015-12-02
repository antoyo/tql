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

/// Analyzer for the join() method.

use syntax::codemap::Spanned;

use ast::{Expression, Join};
use error::{SqlResult, res};
use state::{SqlTable, get_primary_key_field, tables_singleton};
use super::{check_field, mismatched_types, no_primary_key, path_expr_to_identifier};
use types::Type;

/// Convert an `Expression` to a `Join`
pub fn argument_to_join(arg: &Expression, table: &SqlTable) -> SqlResult<Join> {
    let mut errors = vec![];
    let mut join = Join::default();

    if let Some(identifier) = path_expr_to_identifier(arg, &mut errors) {
        check_field(&identifier, arg.span, table, &mut errors);
        match table.fields.get(&identifier) {
            Some(&Spanned { node: ref field_type, .. }) => {
                if let Type::Custom(ref related_table_name) = *field_type {
                    let sql_tables = tables_singleton();
                    if let Some(related_table) = sql_tables.get(related_table_name) {
                        match get_primary_key_field(related_table) {
                            Some(primary_key_field) =>
                                join = Join {
                                    base_field: identifier,
                                    base_table: table.name.clone(),
                                    joined_field: primary_key_field,
                                    joined_table: related_table_name.clone(),
                                },
                            None => errors.push(no_primary_key(related_table_name, related_table.position)),
                        }
                    }
                    // NOTE: if the field type is not an SQL table, an error is thrown by the
                    // linter.
                }
                else {
                    mismatched_types("ForeignKey<_>", field_type, arg.span, &mut errors);
                }
            },
            None => (), // NOTE: This case is handled by the check_field() call above.
        }
    }
    res(join, errors)
}
