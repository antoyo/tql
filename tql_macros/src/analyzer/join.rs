/// Analyzer for the join() method.

use syntax::codemap::Spanned;

use ast::{Expression, Join};
use error::{SqlResult, res};
use state::{SqlTable, get_primary_key_field, singleton};
use super::{check_field, mismatched_types, no_primary_key, path_expr_to_identifier};
use types::Type;

/// Convert an `Expression` to a `Join`
pub fn argument_to_join(arg: &Expression, table: &SqlTable) -> SqlResult<Join> {
    let mut errors = vec![];
    let mut join = Join {
        left_field: "".to_owned(),
        left_table: "".to_owned(),
        right_field: "".to_owned(),
        right_table: "".to_owned(),
    };

    if let Some(identifier) = path_expr_to_identifier(arg, &mut errors) {
        check_field(&identifier, arg.span, table, &mut errors);
        match table.fields.get(&identifier) {
            Some(&Spanned { node: ref field_type, .. }) => {
                if let &Type::Custom(ref related_table_name) = field_type {
                    let sql_tables = singleton();
                    if let Some(related_table) = sql_tables.get(related_table_name) {
                        match get_primary_key_field(related_table) {
                            Some(primary_key_field) =>
                                join = Join {
                                    left_field: identifier,
                                    left_table: table.name.clone(),
                                    right_field: primary_key_field,
                                    right_table: related_table_name.clone(),
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
