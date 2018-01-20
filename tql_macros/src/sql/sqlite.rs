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

//! The SQLite code generator.

use quote::Tokens;

use ast::Aggregate;
use sql::{SqlBackend, ToSql};

pub struct SqliteSqlBackend {}

pub fn create_sql_backend() -> SqliteSqlBackend {
    SqliteSqlBackend { }
}

impl ToSql for Aggregate {
    fn to_sql(&self, index: &mut usize) -> String {
        self.sql_function.to_sql(index) + "(" + &self.field.expect("Aggregate field").to_sql(index) + ")"
    }
}

impl SqlBackend for SqliteSqlBackend {
    fn insert_query(&self, table: &str, fields: &[String], values: &[String]) -> Tokens {
        let query =
            format!("INSERT INTO {table}({fields}) VALUES({values})",
            table = table,
            fields = fields.to_sql(&mut 1),
            values = values.to_sql(&mut 1),
            );
        quote! {
            concat!(#query)
        }
    }
}
