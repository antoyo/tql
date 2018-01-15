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

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::Tokens;
use rand::{self, Rng};
use syn::{
    self,
    Expr,
    Field,
    Fields,
    FieldsNamed,
    Ident,
    ItemStruct,
    parse,
};
#[cfg(feature = "unstable")]
use syn::{AngleBracketedGenericArguments, LitStr, Path, TypePath};
#[cfg(feature = "unstable")]
use syn::PathArguments::AngleBracketed;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use ast::{
    Aggregate,
    Join,
    QueryType,
    TypedField,
};
use attribute::{field_ty_to_type, fields_vec_to_hashmap};
use error::{Error, Result, res};
use plugin::{new_ident, string_literal, usize_literal};
use sql::ToSql;
use state::SqlFields;
use string::token_to_string;
use types::{Type, get_type_parameter, get_type_parameter_as_path};
use {
    SqlQueryWithArgs,
    add_error,
    concat_token_stream,
    empty_token_stream,
    typecheck_arguments,
};
#[cfg(feature = "unstable")]
use {
    respan_tokens_with,
    respan_with,
};

/// Create the _create_query() and from_row() method for the table struct.
pub fn table_methods(item_struct: &ItemStruct) -> Tokens {
    let table_ident = &item_struct.ident;
    if let Fields::Named(FieldsNamed { ref named , .. }) = item_struct.fields {
        let mut fields_to_create = vec![];
        let mut primary_key = None;
        let mut pk_idents = vec![];
        let mut pk_tables = vec![];
        for field in named {
            fields_to_create.push(TypedField {
                identifier: field.ident.expect("field ident").to_string(),
                typ: field_ty_to_type(&field.ty).node.to_sql(),
            });
            let typ = token_to_string(&field.ty);
            if let Some(ident) = field.ident {
                if typ == "PrimaryKey" {
                    primary_key = Some(ident);
                }
                else if typ.starts_with("ForeignKey") {
                    if let syn::Type::Path(ref path) = field.ty {
                        let element = path.path.segments.first().expect("first segment of path");
                        let first_segment = element.value();
                        if let Some(path) = get_type_parameter_as_path(&first_segment.arguments) {
                            let type_ident = get_type_parameter(&first_segment.arguments)
                                .expect("get type parameter");
                            if !pk_tables.contains(&path) {
                                pk_idents.push(Ident::from(format!("{}_pk", type_ident).as_str()));
                                pk_tables.push(path);
                            }
                        }
                    }
                }
            }
        }
        let create_query = format!("CREATE TABLE {table} ({fields})",
            table = table_ident,
            fields = fields_to_create.to_sql()
        );

        let field_idents = named.iter()
            .map(|field| field.ty.clone())
            .enumerate();
        let columns = field_idents.map(|(index, typ)| to_row_get(index, typ, false));

        let field_idents = named.iter()
            .map(|field| field.ty.clone())
            .enumerate();
        let related_columns = field_idents.map(|(index, typ)| to_row_get(index, typ, true));

        let field_count = usize_literal(named.len());

        let field_idents = named.iter()
            .map(|field| field.ident.expect("field has name"));
        let field_idents2 = named.iter()
            .map(|field| field.ident.expect("field has name"));

        let field_list = named.iter()
            .map(|field| {
                format!("{table}.{column} AS \"{table}.{column}\"",
                        column = field.ident.expect("field has name"),
                        table = table_ident
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let field_list = string_literal(&field_list);

        let primary_key =
            if let Some(ident) = primary_key {
                let ident = ident.to_string();
                quote! {
                    #ident
                }
            }
            else {
                quote! {
                    unreachable!("no primary key")
                }
            };
        quote! {
            unsafe impl ::tql::SqlTable for #table_ident {
                const FIELD_COUNT: usize = #field_count;

                fn _create_query() -> String {
                    format!(#create_query #(, #pk_idents = #pk_tables::_primary_key_field())*)
                }

                // TODO: rename to avoid clash.
                fn default() -> Self {
                    unimplemented!()
                }

                #[allow(unused)]
                fn from_row(row: &::postgres::rows::Row) -> Self {
                    Self {
                        #(#field_idents: #columns,)*
                    }
                }

                #[allow(unused)]
                fn from_related_row(row: &::postgres::rows::Row, delta: usize) -> Self {
                    Self {
                        #(#field_idents2: #related_columns,)*
                    }
                }

                #[allow(unused)]
                fn field_list() -> &'static str {
                    #field_list
                }
            }

            impl #table_ident {
                pub fn _primary_key_field() -> &'static str {
                    #primary_key
                }

            }
        }
    }
    else {
        unreachable!("Check is done in get_struct_fields()")
    }
}

/// Add the postgres::types::ToSql implementation on the struct.
/// Its SQL representation is the same as the primary key SQL representation.
pub fn tosql_impl(item_struct: &ItemStruct, primary_key_field: &str) -> Tokens {
    let table_ident = &item_struct.ident;
    let debug_impl = create_debug_impl(item_struct);
    let primary_key_ident = Ident::from(primary_key_field);
    quote! {
        #debug_impl

        impl ::postgres::types::ToSql for #table_ident {
            fn to_sql(&self, ty: &::postgres::types::Type, out: &mut Vec<u8>) ->
                Result<::postgres::types::IsNull, Box<::std::error::Error + 'static + Sync + Send>>
                {
                    self.#primary_key_ident.to_sql(ty, out)
                }

            fn accepts(ty: &::postgres::types::Type) -> bool {
                *ty == ::postgres::types::INT4
            }

            fn to_sql_checked(&self, ty: &::postgres::types::Type, out: &mut ::std::vec::Vec<u8>)
                -> ::std::result::Result<::postgres::types::IsNull,
                Box<::std::error::Error + ::std::marker::Sync + ::std::marker::Send>>
                {
                    ::postgres::types::__to_sql_checked(self, ty, out)
                }
        }

        impl #table_ident {
            #[allow(dead_code)]
            pub fn to_owned(&self) -> Option<Self> {
                unimplemented!();
            }
        }
    }
}

fn create_debug_impl(item_struct: &ItemStruct) -> Tokens {
    let table_ident = &item_struct.ident;
    let table_name = table_ident.to_string();
    if let Fields::Named(FieldsNamed { ref named , .. }) = item_struct.fields {
        let field_idents = named.iter()
            .map(|field| field.ident.expect("field has name"));
        let field_names = field_idents
            .map(|ident| ident.to_string());
        let field_idents = named.iter()
            .map(|field| field.ident.expect("field has name"));
        quote! {
            impl ::std::fmt::Debug for #table_ident {
                fn fmt(&self, formatter: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
                    formatter.debug_struct(#table_name)
                        #(.field(#field_names, &self.#field_idents))*
                        .finish()
                }
            }
        }
    }
    else {
        unreachable!("Check is done in get_struct_fields()")
    }
}

pub fn generate_errors(errors: Vec<Error>) -> TokenStream {
    let mut compiler_errors = quote! {};
    for error in errors {
        add_error(error, &mut compiler_errors);
    }
    #[cfg(not(feature = "unstable"))]
    {
        (quote! {
            #compiler_errors
        }).into()
    }
    #[cfg(feature = "unstable")]
    {
        let expr = LitStr::new("", Span::call_site());
        let gen = quote! {
            #expr
        };
        gen.into()
    }
}

/// Generate the Rust code from the SQL query.
pub(crate) fn gen_query(args: SqlQueryWithArgs) -> TokenStream {
    let ident = Ident::new("connection", args.table_name.span);
    let struct_expr = create_struct(&args.table_name, &args.joins);
    let (aggregate_struct, aggregate_expr) = gen_aggregate_struct(&args.aggregates);
    let args_expr = typecheck_arguments(&args);
    let tokens = gen_query_expr(ident, args, args_expr, struct_expr, aggregate_struct, aggregate_expr);
    tokens.into()
}

/// Generate the Rust code using the `postgres` library depending on the `QueryType`.
fn gen_query_expr(connection_ident: Ident, args: SqlQueryWithArgs, args_expr: Tokens, struct_expr: Tokens,
                  aggregate_struct: Tokens, aggregate_expr: Tokens) -> Tokens
{
    let table_ident = &args.table_name;
    let result_ident = Ident::from("result");
    let sql_query = string_literal(&args.sql);
    let trait_ident = quote_spanned! { table_ident.span() =>
        ::tql::SqlTable
    };
    let pk =
        if args.use_pk {
            quote! {
                , pk = #table_ident::_primary_key_field()
            }
        }
        else {
            quote! { }
        };
    let joins = args.joins.iter()
        .map(|join| {
            let base_field = &join.base_field;
            let related_pk = Ident::new(&format!("{}_pk", join.base_field), join.base_field.span());
            let macro_name = Ident::new(&format!("tql_{}_related_tables", table_ident), Span::call_site());
            let pks_macro_name = Ident::new(&format!("tql_{}_related_pks", table_ident), Span::call_site());
            let code = quote! {
                , #base_field = #macro_name!(#base_field)
                , #related_pk = #pks_macro_name!(#base_field)
            };
            #[cfg(feature = "unstable")]
            let code = respan_tokens_with(code, base_field.span().unstable());
            code
        });
    let join_fields = args.joins.iter()
        .map(|join| {
            let base_field = join.base_field;
            let field_list_macro_name = Ident::new(&format!("tql_{}_field_list", table_ident), Span::call_site());
            quote_spanned! { table_ident.span() =>
                #field_list_macro_name!(#base_field)
            }
        });
    let fields = quote! { <#table_ident as #trait_ident>::field_list() };
    let select_query = || {
        let pks =
            if args.use_pk || !args.joins.is_empty() {
                quote! {
                    #pk
                    #(#joins)*
                }
            }
            else {
                quote! { }
            };
        quote! {
            &format!(#sql_query #pks, fields = [#fields #(, #join_fields)*].join(", "))
        }
    };

    match args.query_type {
        QueryType::AggregateMulti => {
            let result = quote! {{
                let result = #connection_ident.prepare(#sql_query).expect("prepare query");
                result.query(&#args_expr).expect("execute query").iter()
            }};
            let call = quote! {
                .map(|row| {
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
                let result = #connection_ident.prepare(#sql_query).expect("prepare query");
                result.query(&#args_expr).expect("execute query").iter().next().map(|row| {
                    #aggregate_expr
                })
            }}
        },
        QueryType::Create => {
            quote! {{
                #connection_ident.prepare(&<#table_ident as #trait_ident>::_create_query())
                    .and_then(|result| result.execute(&[]))
            }}
        },
        QueryType::InsertOne => {
            let sql_query =
                if args.use_pk {
                    quote! {
                        &format!(#sql_query,
                            returning_pk = format!("RETURNING {}", #table_ident::_primary_key_field()))
                    }
                }
                else {
                    quote! { #sql_query }
                };
            quote! {{
                #connection_ident.prepare(#sql_query)
                    .and_then(|result| {
                        // NOTE: The query is not supposed to fail, hence expect().
                        let rows = result.query(&#args_expr).expect("execute query");
                        // NOTE: There is always one result (the inserted id), hence unwrap().
                        let row = rows.iter().next().unwrap();
                        let count: i32 = row.get(0);
                        Ok(count)
                    })
            }}
        },
        QueryType::SelectMulti => {
            let sql_query = select_query();
            let result =
                quote! {
                    let #result_ident = #connection_ident.prepare(#sql_query).expect("prepare query");
                    let #result_ident = #result_ident.query(&#args_expr).expect("execute query");
                    let results = #result_ident.iter();
                };
            let call = quote! {
                .map(|row| {
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
            let sql_query = select_query();
            let result =
                quote! {
                    let #result_ident = #connection_ident.prepare(#sql_query).expect("prepare query");
                    let #result_ident = #result_ident.query(&#args_expr).expect("execute query");
                    let results = #result_ident.iter().next();
                };
            let call = quote! {
                .map(|row| {
                    #struct_expr
                })
            };
            quote! {{
                #result
                results#call
            }}
        },
        QueryType::Exec => {
            let sql_query =
                if args.use_pk {
                    quote! { &format!(#sql_query #pk) }
                }
                else {
                    quote! { #sql_query }
                };
            quote! {{
                #connection_ident.prepare(#sql_query)
                    .and_then(|result| result.execute(&#args_expr))
            }}
        },
    }
}

/// Create the struct expression needed by the generated code.
fn create_struct(table_ident: &Ident, joins: &[Join]) -> Tokens {
    let row_ident = quote! { row };
    let assign_related_fields =
        joins.iter()
            .map(|join| {
                let ident = &join.base_field;
                quote_spanned! { ident.span() => {
                    let ref mut _related_field: Option<_> = item.#ident;
                    _tql_delta += ::tql::from_related_row(_related_field, &#row_ident, _tql_delta);
                }}
            });
    quote_spanned! { table_ident.span() => {
        #[allow(unused_mut)]
        let mut item = <#table_ident as ::tql::SqlTable>::from_row(&#row_ident);
        let mut _tql_delta = <#table_ident as ::tql::SqlTable>::FIELD_COUNT;
        #(#assign_related_fields)*
        item
    }}
}

/// Generate the aggregate struct and struct expression.
fn gen_aggregate_struct(aggregates: &[Aggregate]) -> (Tokens, Tokens) {
    let mut aggregate_field_idents = vec![];
    let mut aggregate_field_values = vec![];
    let mut def_field_idents = vec![];
    for (index, aggregate) in aggregates.iter().enumerate() {
        let field_name = aggregate.result_name.clone();
        aggregate_field_idents.push(field_name.clone());
        aggregate_field_values.push(quote! { row.get(#index) });
        def_field_idents.push(field_name);
    }
    let struct_ident = new_ident("Aggregate");
    (quote! {
        struct #struct_ident {
            #(#def_field_idents: i32),* // TODO: choose the type from the field?
        }
    },
    quote! {
        #struct_ident {
            #(#aggregate_field_idents: #aggregate_field_values),*
        }
    })
}

/// Get the fields from the struct (also returns the ToSql implementations to check that the types
/// used for ForeignKey have a #[derive(SqlTable)]).
/// Also check if the field types from the struct are supported types.
pub fn get_struct_fields(item_struct: &ItemStruct) -> (Result<SqlFields>, Option<String>, TokenStream) {
    fn error(span: Span, typ: &str) -> Error {
        Error::new_with_code(&format!("use of unsupported type name `{}`", typ),
            span, "E0412")
    }

    let mut primary_key_field = None;
    let position = item_struct.ident.span;
    let mut impls: TokenStream = quote! {}.into();
    let mut errors = vec![];

    let fields: Vec<Field> =
        match item_struct.fields {
            Fields::Named(FieldsNamed { ref named , .. }) => named.into_iter().cloned().collect(),
            _ => return (Err(vec![Error::new("Expected normal struct, found", position)]), None, empty_token_stream()), // TODO: improve this message.
        };
    let mut primary_key_count = 0;
    for field in &fields {
        if let Some(field_ident) = field.ident {
            #[cfg(feature = "unstable")]
            let field_type = &field.ty;
            let field_name = field_ident.to_string();
            let field = field_ty_to_type(&field.ty);
            match field.node {
                Type::Nullable(ref inner_type) => {
                    if let Type::UnsupportedType(ref typ) = **inner_type {
                        errors.push(error(field.span, typ));
                    }
                },
                Type::UnsupportedType(ref typ) =>
                    errors.push(error(field.span, typ)),
                // NOTE: Other types are supported.
                Type::Serial => {
                    primary_key_field = Some(field_name);
                    primary_key_count += 1;
                },
                Type::Custom(ref typ) => {
                    let type_ident = new_ident(typ);
                    let struct_ident = new_ident(&format!("CheckForeignKey{}", rand_string()));
                    // TODO: replace with a trait bound on ForeignKey when it is stable.
                    #[cfg(feature = "unstable")]
                    let mut code: TokenStream;
                    #[cfg(not(feature = "unstable"))]
                    let code: TokenStream;
                    code = quote! {
                        #[allow(dead_code)]
                        struct #struct_ident where #type_ident: ::tql::SqlTable {
                            field: #type_ident,
                        }
                    }.into();
                    #[cfg(feature = "unstable")]
                    {
                        let field_pos =
                            if let syn::Type::Path(TypePath { path: Path { ref segments, .. }, ..}) = *field_type {
                                let segment = segments.first().expect("first segment").into_value();
                                if let AngleBracketed(AngleBracketedGenericArguments { ref args, .. }) =
                                    segment.arguments
                                {
                                    args.first().expect("first argument").span()
                                }
                                else {
                                    field_type.span()
                                }
                            }
                            else {
                                field_type.span()
                            };
                        let span = field_pos.unstable();
                        // NOTE: position the trait at this position so that the error message points
                        // on the type.
                        code = respan_with(code, span);
                    }
                    impls = concat_token_stream(impls, code);
                },
                _ => (),
            }
        }
    }

    match primary_key_count {
        0 => errors.insert(0, Error::new_warning("No primary key found", position)),
        1 => (), // One primary key is OK.
        _ => errors.insert(0, Error::new_warning("More than one primary key is currently not supported", position)),
    }

    let fields = fields_vec_to_hashmap(&fields);
    (res(fields, errors), primary_key_field, impls)
}

/// Create the insert macro for the table struct to check that all the mandatory fields are
/// provided.
pub fn table_macro(item_struct: &ItemStruct) -> Tokens {
    let table_ident = &item_struct.ident;
    let mut primary_key_found = false;
    if let Fields::Named(FieldsNamed { ref named , .. }) = item_struct.fields {
        let mut mandatory_fields = vec![];
        let mut related_table_names = vec![];
        let mut non_related_table_names = vec![];
        let mut related_tables = vec![];
        let mut related_table_types = vec![];
        let mut compiler_errors = vec![];
        let mut fk_patterns = vec![];
        for field in named {
            let typ = token_to_string(&field.ty);
            if let Some(ident) = field.ident {
                if typ == "PrimaryKey" {
                    primary_key_found = true;
                }
                if !typ.starts_with("Option") && typ != "PrimaryKey" {
                    mandatory_fields.push(ident);
                }
                if typ.starts_with("ForeignKey") {
                    if let syn::Type::Path(ref path) = field.ty {
                        let element = path.path.segments.first().expect("first segment of path");
                        let first_segment = element.value();
                        let typ = get_type_parameter(&first_segment.arguments)
                            .expect("ForeignKey inner type");
                        related_tables.push(typ);
                        related_table_names.push(ident);
                        related_table_types.push(get_type_parameter_as_path(&first_segment.arguments)
                            .expect("ForeignKey inner type"));
                        if let Some(path) = get_type_parameter_as_path(&first_segment.arguments) {
                            fk_patterns.push(quote_spanned! { table_ident.span() =>
                                (#ident) => { <#path as ::tql::SqlTable>::field_list() };
                            });
                        }
                    }
                }
                else {
                    non_related_table_names.push(ident);
                    let msg = string_literal(&format!("mismatched types
expected type `ForeignKey<_>`
   found type `{}`", typ));
                    compiler_errors.push(quote_spanned! { field.span() =>
                        compile_error!(#msg)
                    });
                }
            }
        }

        let macro_name = Ident::new(&format!("tql_{}_check_missing_fields", table_ident), Span::call_site());
        let pk_macro_name = Ident::new(&format!("tql_{}_check_primary_key", table_ident), Span::call_site());
        #[cfg(feature = "unstable")]
        let macro_call = quote_spanned! { table_ident.span() =>
            tql_macros::check_missing_fields!
        };
        #[cfg(not(feature = "unstable"))]
        let macro_call = quote! {
            check_missing_fields!
        };
        let pk_code =
            if primary_key_found {
                quote! {}
            }
            else {
                quote! {
                    compiler_error!("No primary key found")
                }
            };
        let related_tables_macro_name = Ident::new(&format!("tql_{}_related_tables", table_ident), Span::call_site());
        let related_pks_macro_name = Ident::new(&format!("tql_{}_related_pks", table_ident), Span::call_site());
        let field_list_macro_name = Ident::new(&format!("tql_{}_field_list", table_ident), Span::call_site());
        let related_table_names2 = &related_table_names;
        let related_table_names = &related_table_names;
        quote_spanned! { table_ident.span() =>
            #[macro_export]
            macro_rules! #macro_name {
                ($($insert_idents:ident),*) => {
                    #macro_call([#(#mandatory_fields),*], [$($insert_idents),*])
                };
            }

            #[macro_export]
            macro_rules! #pk_macro_name {
                () => {
                    #pk_code
                };
            }

            #[allow(unused_macros)]
            macro_rules! #related_tables_macro_name {
                #((#related_table_names) => { #related_tables };)*
                #((#non_related_table_names) => { #compiler_errors };)*
                // NOTE: the check for the field name is done elsewhere, hence it is okay to return
                // "" here.
                ($tt:tt) => { "" };
            }

            #[allow(unused_macros)]
            macro_rules! #related_pks_macro_name {
                #((#related_table_names2) => { #related_table_types::_primary_key_field() };)*
                // NOTE: the check for the field name is done elsewhere, hence it is okay to return
                // "" here.
                ($tt:tt) => { "" };
            }

            #[allow(unused_macros)]
            macro_rules! #field_list_macro_name {
                #(#fk_patterns)*
                ($tt:tt) => { "" };
            }
        }
    }
    else {
        unreachable!("Check is done in get_struct_fields()")
    }
}

fn to_row_get(index: usize, typ: syn::Type, with_delta: bool) -> Tokens {
    if let syn::Type::Path(path) = typ {
        let segment = path.path.segments.first().expect("first segment").into_value();
        if segment.ident == "ForeignKey" {
            return quote! {
                None
            };
        }
    }
    let index = usize_literal(index);
    let index =
        if with_delta {
            quote! {
                #index + delta
            }
        }
        else {
            quote! { #index }
        };
    quote! {
        row.get(#index)
    }
}

struct Arguments(Punctuated<Expr, Token![,]>);

impl syn::synom::Synom for Arguments {
    // call!(Punctuated::parse_terminated) will parse a terminated sequence of
    // Synom objects. Expr implements synom so we're good.
    named!(parse -> Self, map!(call!(Punctuated::parse_terminated), Arguments));
}

pub fn gen_check_missing_fields(input: TokenStream) -> TokenStream {
    let args: Arguments = parse(input).expect("parse check_missing_fields!()");
    let args = args.0;
    let arg1 = &args[0];
    let arg2 = &args[1];
    let mut mandatory_fields = vec![];
    let mut fields = vec![];

    if let Expr::Array(ref array) = *arg1 {
        for elem in &array.elems {
            if let Expr::Path(ref path) = *elem {
                mandatory_fields.push(path.path.segments[0].ident.clone());
            }
        }
    }

    if let Expr::Array(ref array) = *arg2 {
        for elem in &array.elems {
            let path =
                if let Expr::Group(ref group) = *elem {
                    if let Expr::Path(ref path) = *group.expr {
                        path
                    }
                    else {
                        panic!("Expecting path");
                    }
                }
                // NOTE: need this condition on stable.
                else if let Expr::Path(ref path) = *elem {
                    path
                }
                else {
                    panic!("Expecting path");
                };
            fields.push(path.path.segments[0].ident.clone());
        }
    }

    let mut missing_fields = vec![];

    for field in mandatory_fields {
        if !fields.contains(&field) {
            missing_fields.push(field.to_string());
        }
    }

    if !missing_fields.is_empty() {
        let missing_fields = missing_fields.join(", ");
        let error = string_literal(&format!("missing fields: {}", missing_fields));

        (quote! {
            compile_error!(#error);
        }).into()
    }
    else {
        empty_token_stream()
    }
}

fn rand_string() -> String {
    rand::thread_rng().gen_ascii_chars().take(30).collect()
}
