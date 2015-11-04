/// Analyzer for the join() method.

use syntax::codemap::Spanned;

use ast::{Expression, Join};
use error::{Error, SqlResult, res};
use state::{SqlFields, get_primary_key_field_by_table_name};
use super::{check_field, no_primary_key, path_expr_to_identifier};
use types::Type;

/// Convert an `Expression` to a `Join`
pub fn argument_to_join(arg: &Expression, table_name: &str, table: &SqlFields) -> SqlResult<Join> {
    let mut errors = vec![];
    let mut join = Join {
        left_field: "".to_owned(),
        left_table: "".to_owned(),
        right_field: "".to_owned(),
        right_table: "".to_owned(),
    };

    if let Some(identifier) = path_expr_to_identifier(arg, &mut errors) {
        check_field(&identifier, arg.span, table_name, table, &mut errors);
        match table.get(&identifier) {
            Some(&Spanned { node: ref field_type, .. }) => {
                if let &Type::Custom(ref related_table_name) = field_type {
                    match get_primary_key_field_by_table_name(related_table_name) {
                        Some(primary_key_field) =>
                            join = Join {
                                left_field: identifier,
                                left_table: table_name.to_owned(),
                                right_field: primary_key_field,
                                right_table: related_table_name.clone(),
                            },
                        None => errors.push(no_primary_key(related_table_name, arg.span)), // TODO: utiliser la vraie position.
                    }
                }
                else {
                    errors.push(Error::new_with_code(
                        format!("mismatched types:\n expected `ForeignKey<_>`,\n    found `{}`", field_type),
                        arg.span,
                        "E0308",
                    ));
                }
            },
            None => (), // This case is handled by the check_field() call above.
        }
    }
    res(join, errors)
}
