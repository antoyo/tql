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

use proc_macro2::{Span,TokenStream};
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
    fn convert_index(&self, index: usize) -> TokenStream {
        let index = index as i32;
        quote! {
            #index
        }
    }

    fn delta_type(&self) -> TokenStream {
        quote! { i32 }
    }

    fn gen_query_expr(&self, connection_expr: TokenStream, args: &SqlQueryWithArgs, args_expr: TokenStream, struct_expr: TokenStream,
                      aggregate_struct: TokenStream, aggregate_expr: TokenStream) -> TokenStream
    {
        let result_ident = Ident::new("__tql_result",Span::call_site());
        let sql_query = &args.sql;
        let rusqlite_ident = quote_spanned! { connection_expr.span() =>
            ::rusqlite
        };

        match args.query_type {
            QueryType::AggregateMulti => {
                quote! {{
                    #aggregate_struct

                    #connection_expr.prepare(#sql_query)
                        .and_then(|mut #result_ident| {
                            let #result_ident = #result_ident.query_map(&#args_expr, |__tql_item_row| {
                                    #aggregate_expr
                                })?;
                            #result_ident.collect::<Result<Vec<_>, _>>()
                                // TODO: return an iterator instead of a vector.
                        })
                }}
            },
            QueryType::AggregateOne => {
                quote! {{
                    #aggregate_struct

                    #connection_expr.prepare(#sql_query)
                        .and_then(|mut #result_ident| {
                            #result_ident.query_map(&#args_expr, |__tql_item_row| {
                                    #aggregate_expr
                                })?
                                .next()
                                .ok_or_else(|| #rusqlite_ident::Error::QueryReturnedNoRows)?
                        })
                }}
            },
            QueryType::Create => {
                quote! {
                    #connection_expr.prepare(#sql_query)
                        .and_then(|mut result| result.execute(&[]))
                }
            },
            QueryType::InsertOne => {
                quote! {
                    #connection_expr.prepare(#sql_query)
                        .and_then(|mut result| result.execute(&#args_expr))
                        .map(|_| #connection_expr.last_insert_rowid() as i32) // FIXME: don't cast?
                }
            },
            QueryType::SelectMulti => {
                quote! {
                    #connection_expr.prepare(#sql_query)
                        .and_then(|mut #result_ident| {
                            let #result_ident = #result_ident.query_map(&#args_expr, |__tql_item_row| {
                                    #struct_expr
                                })?;
                            #result_ident.collect::<Result<Vec<_>, _>>()
                            // TODO: return an iterator instead of a vector.
                        })
                }
            },
            QueryType::SelectOne => {
                quote! {
                    #connection_expr.prepare(#sql_query)
                        .and_then(|mut #result_ident| {
                            #result_ident.query_map(&#args_expr, |__tql_item_row| {
                                    #struct_expr
                                })?
                                .next()
                                .ok_or_else(|| #rusqlite_ident::Error::QueryReturnedNoRows)?
                        })
                }
            },
            QueryType::Exec => {
                quote! {
                    #connection_expr.prepare(#sql_query)
                        .and_then(|mut result| result.execute(&#args_expr))
                }
            },
        }
    }

    fn int_literal(&self, num: usize) -> Expr {
        Expr::Lit(ExprLit {
            attrs: vec![],
            lit: Lit::Int(LitInt::new(num as u64, IntSuffix::I32, Span::call_site())),
        })
    }

    fn row_type_ident(&self, table_ident: &Ident) -> TokenStream {
        quote_spanned! { table_ident.span() =>
            ::rusqlite::Row
        }
    }

    fn to_sql(&self, primary_key_ident: &Ident) -> TokenStream {
        quote! {
            self.#primary_key_ident.to_sql()
        }
    }

    fn to_sql_impl(&self, table_ident: &Ident, to_sql_code: TokenStream) -> TokenStream {
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
