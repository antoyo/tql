//! A conversion function for the attribute.

use std::collections::BTreeMap;

use syntax::ast::{AngleBracketedParameterData, FieldIter, Path, StructFieldKind, Ty};
use syntax::ast::PathParameters::AngleBracketedParameters;
use syntax::ast::Ty_::TyPath;

use state::{SqlFields, Type};

fn field_ty_to_type(ty: &Ty) -> Type {
    let mut typ = Type::Dummy;
    if let TyPath(None, Path { ref segments, .. }) = ty.node {
        if segments.len() == 1 {
            let ident = segments[0].identifier.to_string();
            if ident == "String" {
                typ = Type::String;
            }
            else if ident == "i32" {
                typ = Type::I32;
            }
            // TODO
            // else if ident == "" {
            // }
            else if ident == "ForeignKey" {
                if let AngleBracketedParameters(AngleBracketedParameterData { ref types, .. }) = segments[0].parameters {
                    match types.first() {
                        Some(ty) => {
                            if let TyPath(None, Path { ref segments, .. }) = ty.node {
                                typ = Type::Custom(segments[0].identifier.to_string());
                            }
                        },
                        None => (), // TODO
                    }
                }
            }
        }
    }
    typ
}

/// Convert a vector of Rust struct fields to a collection of fields.
pub fn fields_vec_to_hashmap(fields: FieldIter) -> SqlFields {
    let mut sql_fields = BTreeMap::new();
    // TODO: ajouter le champ id.
    //sql_fields.insert("id".to_string(), Type::Int);
    for field in fields.into_iter() {
        if let StructFieldKind::NamedField(ident, _) = field.node.kind {
            sql_fields.insert(ident.to_string(), field_ty_to_type(&*field.node.ty));
        }
    }
    sql_fields
}
