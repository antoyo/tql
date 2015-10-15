//! A conversion function for the attribute.

use std::collections::BTreeMap;

use syntax::ast::{AngleBracketedParameterData, Path, StructField_, StructFieldKind, Ty};
use syntax::ast::PathParameters::AngleBracketedParameters;
use syntax::ast::Ty_::TyPath;
use syntax::codemap::Spanned;

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
                match segments[0].parameters {
                    AngleBracketedParameters(AngleBracketedParameterData { ref types, .. }) => {
                        match types.first() {
                            Some(ty) => {
                                if let TyPath(None, Path { ref segments, .. }) = ty.node {
                                    typ = Type::Custom(segments[0].identifier.to_string());
                                }
                            },
                            None => (), // TODO
                        }
                    },
                    _ => {
                        // TODO
                    },
                }
            }
        }
    }
    typ
}

/// Convert a vector of Rust struct fields to a collection of fields.
pub fn fields_vec_to_hashmap(fields: &Vec<Spanned<StructField_>>) -> SqlFields {
    let mut sql_fields = BTreeMap::new();
    // TODO: ajouter le champ id.
    //sql_fields.insert("id".to_string(), Type::Int);
    for field in fields {
        if let StructFieldKind::NamedField(ident, _) = field.node.kind {
            sql_fields.insert(ident.to_string(), field_ty_to_type(&*field.node.ty));
        }
    }
    sql_fields
}
