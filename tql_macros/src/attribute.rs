//! A conversion function for the attribute.

use std::collections::BTreeMap;
use std::fmt::Write;

use syntax::ast::{AngleBracketedParameters, AngleBracketedParameterData, FieldIter, StructFieldKind, Ty};
use syntax::ast::Ty_::TyPath;
use syntax::codemap::Spanned;

use state::SqlFields;
use types::Type;

/// Convert a type from the Rust AST to the SQL `Type`.
fn field_ty_to_type(ty: &Ty) -> Spanned<Type> {
    let typ =
        if let TyPath(None, ref path) = ty.node {
            Type::from(path)
        }
        else {
            let mut type_string = String::new();
            let _ = write!(type_string, "{:?}", ty);
            Type::UnsupportedType(type_string[5..type_string.len() - 1].to_owned())
        };
    let mut position = ty.span;
    if let TyPath(_, ref path) =  ty.node {
        if path.segments[0].identifier.to_string() == "Option" {
            if let AngleBracketedParameters(AngleBracketedParameterData { ref types, .. }) = path.segments[0].parameters {
                if let Some(typ) = types.first() {
                    position = typ.span
                }
            }
        }
    }
    Spanned {
        node: typ,
        span: position,
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
