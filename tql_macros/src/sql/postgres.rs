/*
 * Copyright (c) 2017-2018 Boucher, Antoni <bouanto@zoho.com>
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

//! The PostgreSQL code generator.

use proc_macro2::Span;
use quote::Tokens;
use syn::Ident;

use ast::Aggregate;
use sql::{SqlBackend, ToSql, string_token};

pub struct PostgresSqlBackend {}

pub fn create_sql_backend() -> PostgresSqlBackend {
    PostgresSqlBackend { }
}

impl ToSql for Aggregate {
    fn to_sql(&self, index: &mut usize) -> String {
        // TODO: do not hard-code the type.
        "CAST(".to_string() + &self.sql_function.to_sql(index) + "(" +
            &self.field.expect("Aggregate field").to_sql(index) + ") AS DOUBLE PRECISION)"
    }
}

impl SqlBackend for PostgresSqlBackend {
    fn insert_query(&self, table: &str, fields: &[String], values: &[String]) -> Tokens {
        let query_start =
            format!("INSERT INTO {table}({fields}) VALUES({values}) RETURNING ",
            table = table,
            fields = fields.to_sql(&mut 1),
            values = values.to_sql(&mut 1),
            );
        let query_start = string_token(&query_start);
        let macro_name = Ident::new(format!("tql_{}_primary_key_field", table).as_str(), Span::call_site());
        quote! {
            concat!(#query_start, #macro_name!())
        }
    }
}
