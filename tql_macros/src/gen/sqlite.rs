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

use proc_macro2::Span;
use quote::Tokens;
use syn::{
    Expr,
    ExprLit,
    Ident,
    IntSuffix,
    Lit,
    LitInt,
};
use syn::spanned::Spanned;

use ast::QueryType;
use super::BackendGen;
use SqlQueryWithArgs;

pub struct SqliteBackend {}

pub fn create_backend() -> SqliteBackend {
    SqliteBackend {
    }
}

impl BackendGen for SqliteBackend {
    fn convert_index(&self, index: usize) -> Tokens {
        let index = index as i32;
        quote! {
            #index
        }
    }

    fn delta_type(&self) -> Tokens {
        quote! { i32 }
    }

    fn gen_query_expr(&self, connection_expr: Tokens, args: SqlQueryWithArgs, args_expr: Tokens, struct_expr: Tokens,
                      aggregate_struct: Tokens, aggregate_expr: Tokens) -> Tokens
    {
        let result_ident = Ident::from("result");
        let sql_query = &args.sql;

        match args.query_type {
            QueryType::AggregateMulti => {
                quote! {{
                    #aggregate_struct

                    let mut #result_ident = #connection_expr.prepare(#sql_query);
                    let mut #result_ident = #result_ident.expect("prepare query");
                    let mut #result_ident = #result_ident.query_map(&#args_expr, |__tql_item_row| {
                            #aggregate_expr
                        });
                    let mut #result_ident = #result_ident.expect("execute query")
                        .map(|item| item.expect("item selection")); // TODO: do better error handling.
                    #result_ident.collect::<Vec<_>>()
                        // TODO: return an iterator instead of a vector.
                }}
            },
            QueryType::AggregateOne => {
                quote! {{
                    #aggregate_struct

                    let mut #result_ident = #connection_expr.prepare(#sql_query)
                        .expect("prepare query");
                    let mut #result_ident = #result_ident.query_map(&#args_expr, |__tql_item_row| {
                            #aggregate_expr
                        })
                        .expect("execute query")
                        .next();
                    #result_ident.map(|__tql_item| __tql_item.expect("next row")) // TODO: do better error handling.
                }}
            },
            QueryType::Create => {
                quote! {{
                    #connection_expr.prepare(#sql_query)
                        .and_then(|mut result| result.execute(&[]))
                }}
            },
            QueryType::InsertOne => {
                quote! {{
                    #connection_expr.prepare(#sql_query)
                        .and_then(|mut result| result.execute(&#args_expr))
                        .map(|_| #connection_expr.last_insert_rowid() as i32) // FIXME: don't cast?
                }}
            },
            QueryType::SelectMulti => {
                quote! {{
                    let mut #result_ident = #connection_expr.prepare(#sql_query).expect("prepare query");
                    let mut #result_ident = #result_ident.query_map(&#args_expr, |__tql_item_row| {
                            #struct_expr
                        });
                    let mut #result_ident = #result_ident.expect("execute query")
                        .map(|item| item.expect("item selection")); // TODO: do better error handling.
                    #result_ident.collect::<Vec<_>>()
                        // TODO: return an iterator instead of a vector.
                }}
            },
            QueryType::SelectOne => {
                quote! {{
                    let mut #result_ident = #connection_expr.prepare(#sql_query)
                        .expect("prepare query");
                    let mut #result_ident = #result_ident.query_map(&#args_expr, |__tql_item_row| {
                            #struct_expr
                        })
                        .expect("execute query")
                        .next();
                    #result_ident.map(|__tql_item| __tql_item.expect("next row"))
                }}
            },
            QueryType::Exec => {
                quote! {{
                    #connection_expr.prepare(#sql_query)
                        .and_then(|mut result| result.execute(&#args_expr))
                }}
            },
        }
    }

    fn int_literal(&self, num: usize) -> Expr {
        Expr::Lit(ExprLit {
            attrs: vec![],
            lit: Lit::Int(LitInt::new(num as u64, IntSuffix::I32, Span::call_site())),
        })
    }

    fn row_type_ident(&self, table_ident: &Ident) -> Tokens {
        quote_spanned! { table_ident.span() =>
            ::rusqlite::Row
        }
    }

    fn to_sql(&self, primary_key_ident: &Ident) -> Tokens {
        quote! {
            self.#primary_key_ident.to_sql()
        }
    }

    fn to_sql_impl(&self, table_ident: &Ident, to_sql_code: Tokens) -> Tokens {
        let rusqlite_ident = quote_spanned! { table_ident.span() =>
            ::rusqlite
        };
        quote! {
            impl #rusqlite_ident::types::ToSql for #table_ident {
                fn to_sql(&self) -> #rusqlite_ident::Result<#rusqlite_ident::types::ToSqlOutput>
                {
                    #to_sql_code
                }
            }
        }
    }
}
