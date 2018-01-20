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

use quote::Tokens;
use syn::{Expr, Ident};
use syn::spanned::Spanned;

use ast::QueryType;
use super::BackendGen;
use SqlQueryWithArgs;

pub struct PostgresBackend {}

pub fn create_backend() -> PostgresBackend {
    PostgresBackend {
    }
}

impl BackendGen for PostgresBackend {
    fn convert_index(&self, index: usize) -> Tokens {
        quote! {
            #index
        }
    }

    fn delta_type(&self) -> Tokens {
        quote! { usize }
    }

    /// Generate the Rust code using the `postgres` library depending on the `QueryType`.
    fn gen_query_expr(&self, connection_expr: Tokens, args: SqlQueryWithArgs, args_expr: Tokens, struct_expr: Tokens,
                      aggregate_struct: Tokens, aggregate_expr: Tokens) -> Tokens
    {
        let result_ident = Ident::from("result");
        let sql_query = &args.sql;

        match args.query_type {
            QueryType::AggregateMulti => {
                let result = quote! {{
                    let result = #connection_expr.prepare(#sql_query).expect("prepare query");
                    result.query(&#args_expr).expect("execute query").iter()
                }};
                let call = quote! {
                    .map(|__tql_item_row| {
                        #aggregate_expr
                    }).collect::<Vec<_>>()
                    // TODO: return an iterator instead of a vector.
                };
                quote! {{
                    #aggregate_struct
                    #result#call
                }}
            },
            QueryType::AggregateOne => {
                quote! {{
                    #aggregate_struct
                    let result = #connection_expr.prepare(#sql_query).expect("prepare query");
                    result.query(&#args_expr).expect("execute query").iter().next().map(|__tql_item_row| {
                        #aggregate_expr
                    })
                }}
            },
            QueryType::Create => {
                quote! {{
                    #connection_expr.prepare(#sql_query)
                        .and_then(|result| result.execute(&[]))
                }}
            },
            QueryType::InsertOne => {
                quote! {{
                    #connection_expr.prepare(#sql_query)
                        .and_then(|result| {
                            // NOTE: The query is not supposed to fail, hence expect().
                            let rows = result.query(&#args_expr).expect("execute query");
                            // NOTE: There is always one result (the inserted id), hence unwrap().
                            let __tql_item_row = rows.iter().next().unwrap();
                            let count: i32 = __tql_item_row.get(0);
                            Ok(count)
                        })
                }}
            },
            QueryType::SelectMulti => {
                let result =
                    quote! {
                        let #result_ident = #connection_expr.prepare(#sql_query).expect("prepare query");
                        let #result_ident = #result_ident.query(&#args_expr).expect("execute query");
                        let results = #result_ident.iter();
                    };
                let call = quote! {
                    .map(|__tql_item_row| {
                        #struct_expr
                    }).collect::<Vec<_>>()
                    // TODO: return an iterator instead of a vector.

                };
                quote! {{
                    #result
                    results#call
                }}
            },
            QueryType::SelectOne => {
                let result =
                    quote! {
                        let #result_ident = #connection_expr.prepare(#sql_query).expect("prepare query");
                        let #result_ident = #result_ident.query(&#args_expr).expect("execute query");
                        let results = #result_ident.iter().next();
                    };
                let call = quote! {
                    .map(|__tql_item_row| {
                        #struct_expr
                    })
                };
                quote! {{
                    #result
                    results#call
                }}
            },
            QueryType::Exec => {
                quote! {{
                    #connection_expr.prepare(#sql_query)
                        .and_then(|result| result.execute(&#args_expr))
                }}
            },
        }
    }

    fn int_literal(&self, num: usize) -> Expr {
        Expr::Lit(ExprLit {
            attrs: vec![],
            lit: Lit::Int(LitInt::new(num as u64, IntSuffix::Usize, Span::call_site())),
        })
    }

    fn row_type_ident(&self, table_ident: &Ident) -> Tokens {
        quote_spanned! { table_ident.span() =>
            ::postgres::rows::Row
        }
    }

    fn to_sql(&self, primary_key_ident: &Ident) -> Tokens {
        quote! {
            self.#primary_key_ident.to_sql(ty, out)
        }
    }

    fn to_sql_impl(&self, table_ident: &Ident, to_sql_code: Tokens) -> Tokens {
        let std_ident = quote_spanned! { table_ident.span() =>
            ::std
        };
        let postgres_ident = quote_spanned! { table_ident.span() =>
            ::postgres
        };
        quote! {
            impl #postgres_ident::types::ToSql for #table_ident {
                fn to_sql(&self, ty: &#postgres_ident::types::Type, out: &mut Vec<u8>) ->
                    Result<#postgres_ident::types::IsNull, Box<#std_ident::error::Error + 'static + Sync + Send>>
                {
                    #to_sql_code
                }

                fn accepts(ty: &#postgres_ident::types::Type) -> bool {
                    *ty == #postgres_ident::types::INT4
                }

                fn to_sql_checked(&self, ty: &#postgres_ident::types::Type, out: &mut #std_ident::vec::Vec<u8>)
                    -> #std_ident::result::Result<#postgres_ident::types::IsNull,
                    Box<#std_ident::error::Error + #std_ident::marker::Sync + #std_ident::marker::Send>>
                {
                    #postgres_ident::types::__to_sql_checked(self, ty, out)
                }
            }
        }
    }
}
