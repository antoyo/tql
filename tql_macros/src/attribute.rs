//! A conversion function for the attribute.

use std::collections::BTreeMap;

use syntax::ast::{AngleBracketedParameterData, FieldIter, Path, StructFieldKind, Ty};
use syntax::ast::PathParameters::AngleBracketedParameters;
use syntax::ast::Ty_::TyPath;
use syntax::codemap::Spanned;

use state::{SqlFields, Type};

/// Convert a type from the Rust AST to the SQL `Type`.
fn field_ty_to_type(ty: &Ty) -> Spanned<Type> {
    let mut typ = Type::UnsupportedType("".to_owned());
    if let TyPath(None, Path { ref segments, .. }) = ty.node {
        if segments.len() == 1 {
            let ident = segments[0].identifier.to_string();
            typ =
                match &ident[..] {
                    "String" => {
                        Type::String
                    },
                    "i32" => {
                        Type::I32
                    },
                    "ForeignKey" => {
                        if let AngleBracketedParameters(AngleBracketedParameterData { ref types, .. }) = segments[0].parameters {
                            match types.first() {
                                Some(ty) => {
                                    if let TyPath(None, Path { ref segments, .. }) = ty.node {
                                        Type::Custom(segments[0].identifier.to_string())
                                    }
                                    else {
                                        Type::UnsupportedType("".to_owned()) // TODO
                                    }
                                },
                                None => Type::UnsupportedType("".to_owned()), // TODO
                            }
                        }
                        else {
                            Type::UnsupportedType("".to_owned()) // TODO
                        }
                    },
                    "PrimaryKey" => {
                        Type::Serial
                    },
                    typ => Type::UnsupportedType(typ.to_owned()),
                };
        }
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
