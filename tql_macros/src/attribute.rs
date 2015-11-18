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

//! A conversion function for the #[SqlTable] attribute.

use std::collections::BTreeMap;
use std::fmt::Write;

use syntax::ast::{AngleBracketedParameters, AngleBracketedParameterData, StructField, StructFieldKind, Ty};
use syntax::ast::Ty_::TyPath;
use syntax::codemap::Spanned;

use state::SqlFields;
use types::Type;

/// Convert a type from the Rust AST to the SQL `Type`.
#[allow(cmp_owned)]
fn field_ty_to_type(ty: &Ty) -> Spanned<Type> {
    let typ =
        if let TyPath(None, ref path) = ty.node {
            Type::from(path)
        }
        else {
            let mut type_string = String::new();
            // TODO: find a better way to get a string representation of a type.
            let _ = write!(type_string, "{:?}", ty);
            Type::UnsupportedType(type_string[5..type_string.len() - 1].to_owned())
        };
    let mut position = ty.span;
    if let TyPath(_, ref path) =  ty.node {
        if path.segments[0].identifier.to_string() == "Option" {
            if let AngleBracketedParameters(AngleBracketedParameterData { ref types, .. }) = path.segments[0].parameters {
                // TODO: use unwrap().
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
pub fn fields_vec_to_hashmap(fields: &[StructField]) -> SqlFields {
    let mut sql_fields = BTreeMap::new();
    for field in fields {
        if let StructFieldKind::NamedField(ident, _) = field.node.kind {
            if !sql_fields.contains_key(&ident.to_string()) {
                sql_fields.insert(ident.to_string(), field_ty_to_type(&*field.node.ty));
            }
            // NOTE: do not override the field type. Rust will show an error if the same field name
            // is used twice.
        }
    }
    sql_fields
}
