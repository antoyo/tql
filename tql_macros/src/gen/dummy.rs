/*
 * Copyright (c) 2018 Boucher, Antoni <bouanto@zoho.com>
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

use proc_macro2::TokenStream;
use syn::{Expr, Ident};

use super::BackendGen;
use SqlQueryWithArgs;

pub struct DummyBackend {}

pub fn create_backend() -> DummyBackend {
    DummyBackend {
    }
}

impl BackendGen for DummyBackend {
    fn convert_index(&self, _index: usize) -> TokenStream {
        unreachable!("Enable one of the following features: sqlite, pg");
    }

    fn delta_type(&self) -> TokenStream {
        unreachable!("Enable one of the following features: sqlite, pg");
    }

    fn gen_query_expr(&self, _connection_expr: TokenStream, _args: &SqlQueryWithArgs, _args_expr: TokenStream, _struct_expr: TokenStream,
                      _aggregate_struct: TokenStream, _aggregate_expr: TokenStream) -> TokenStream
    {
        unreachable!("Enable one of the following features: sqlite, pg");
    }

    fn int_literal(&self, _num: usize) -> Expr {
        unreachable!("Enable one of the following features: sqlite, pg");
    }

    fn row_type_ident(&self, _table_ident: &Ident) -> TokenStream {
        unreachable!("Enable one of the following features: sqlite, pg");
    }

    fn to_sql(&self, _primary_key_ident: &Ident) -> TokenStream {
        unreachable!("Enable one of the following features: sqlite, pg");
    }

    fn to_sql_impl(&self, _table_ident: &Ident, _to_sql_code: TokenStream) -> TokenStream {
        unreachable!("Enable one of the following features: sqlite, pg");
    }
}
