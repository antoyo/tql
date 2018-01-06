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

//! A conversion function for the #[SqlTable] attribute.

use std::collections::BTreeMap;
use std::fmt::Write;

use syn::{
    self,
    AngleBracketedGenericArguments,
    Field,
    PathArguments,
    TypePath,
};
use syn::spanned::Spanned;

use ast::WithSpan;
use state::{BothTypes, SqlFields};
use types::Type;

/// Convert a type from the Rust AST to the SQL `Type`.
//#[allow(cmp_owned)]
pub fn field_ty_to_type(ty: &syn::Type) -> WithSpan<Type> {
    let typ =
        if let syn::Type::Path(TypePath { ref path, .. }) = *ty {
            Type::from(path)
        }
        else {
            let mut type_string = String::new();
            // TODO: find a better way to get a string representation of a type.
            let _ = write!(type_string, "{}", quote! { #ty });
            Type::UnsupportedType(type_string)
        };
    let mut position = ty.span();
    if let syn::Type::Path(TypePath { ref path, .. }) =  *ty {
        if path.segments.first().expect("first segment in path").item().ident.to_string() == "Option" {
            if let PathArguments::AngleBracketed(AngleBracketedGenericArguments { ref args, .. }) = path.segments.first().expect("first segment in path").item().arguments {
                // TODO: use unwrap().
                let element = args.first().expect("first arg");
                let typ = element.item();
                position = typ.span();
            }
        }
    }
    WithSpan {
        node: typ,
        span: position,
    }
}

/// Convert a vector of Rust struct fields to a collection of fields.
pub fn fields_vec_to_hashmap(fields: &[Field]) -> SqlFields {
    let mut sql_fields = BTreeMap::new();
    for field in fields {
        if let Some(ident) = field.ident {
            if !sql_fields.contains_key(&ident) {
                let ty = field_ty_to_type(&field.ty);
                sql_fields.insert(ident, BothTypes {
                    syn_type: field.ty.clone(),
                    ty,
                });
            }
            // NOTE: do not override the field type. Rust will show an error if the same field name
            // is used twice.
        }
    }
    sql_fields
}
