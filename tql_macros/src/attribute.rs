//! A conversion function for the attribute.

use std::collections::BTreeMap;

use syntax::ast::{FieldIter, StructFieldKind, Ty};
use syntax::ast::Ty_::TyPath;
use syntax::codemap::Spanned;

use state::SqlFields;
use types::Type;

/// Convert a type from the Rust AST to the SQL `Type`.
fn field_ty_to_type(ty: &Ty) -> Spanned<Type> {
    let mut typ = Type::UnsupportedType("".to_owned());
    if let TyPath(None, ref path) = ty.node {
        typ = Type::from(path);
    }
    Spanned {
        node: typ,
        span: ty.span,
    }
}

/// Convert a vector of Rust struct fields to a collection of fields.
pub fn fields_vec_to_hashmap(fields: FieldIter) -> SqlFields {
    let mut sql_fields = BTreeMap::new();
    for field in fields.into_iter() {
        if let StructFieldKind::NamedField(ident, _) = field.node.kind {
            sql_fields.insert(ident.to_string(), field_ty_to_type(&*field.node.ty));
        }
    }
    sql_fields
}
