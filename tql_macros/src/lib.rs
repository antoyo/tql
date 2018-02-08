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

/*
 * TODO: check that values are also moved in nightly. If not, fix stable.
 *
 * TODO: return an iterator instead of a Vec.
 * TODO: ManyToMany.
 * TODO: support the missing types
 * (https://docs.rs/postgres/0.15.1/postgres/types/trait.ToSql.html).
 *
 * TODO: error for unsupported types in backends.
 * TODO: remove useless empty string ("") in generated code (concat!("", "")).
 * TODO: avoid using quote_spanned and respan when possible and document all of their usage.
 * TODO: allow using a model from another module without #[macro_use].
 * TODO: write multi-crate test.
 * TODO: show proper error instead of using expect() to parse the content of the macros.
 *
 * TODO: show a better error when using a type that is not a table (both in ForeignKey<_> and in
 * sql!(_.all())).
 * FIXME: escape name like `Table` to avoid error.
 * FIXME: error when having mutiple ForeignKey with the same table (then support having multiple
 * ForeignKey).
 * TODO: document the management of the connection.
 * TODO: use as_ref() for Ident instead of &ident.to_string().
 * TODO: support recursive foreign key.
 * TODO: write fail tests for stable using include!().
 * TODO: try to hide Option in the mismatched type error message for ForeignKey.
 * TODO: use fully-qualified name everywhere in the query (aggregate, …).
 *
 * TODO: support other types (uuid, string) for the primary key, possibly by making it generic.
 * TODO: allow using other fields in filter(), update(), … like F() expressions in Django
 ** Table.filter(field1 > Table.field2) may not work.
 ** Table.filter(field1 > $field2)
 * TODO: unique constraints.
 * TODO: support primary key with multiple columns.
 * TODO: allow selecting only some fields.
 * TODO: join on non foreign key.
 * TODO: allow user-defined functions (maybe with partial query?) and types.
 * TODO: add table_name attribute to allow changing the table name.
 *
 * TODO: remove allow_failure for beta when this issue is fixed:
 * https://github.com/rust-lang/rust/issues/46478
 *
 * TODO: use synom instead of parsing manually?
 * FIXME: error (cannot find macro `tql_Message_check_missing_fields!` in this scope) when putting
 * another custom derive (like Serialize in the chat example) before SqlTable.
 *
 * TODO: improve formatting of the README table.
 * TODO: the error message sometimes show String instead of &str.
 * FIXME: warning should not be errors on stable.
 *
 * TODO: switch to a binding to a C postgresql library for better performance?
 * FIXME: postgres crate seems to be doing too much communication with the server, which might
 * explain why it is slow.
 */

#![cfg_attr(feature = "unstable", feature(proc_macro))]
#![recursion_limit="128"]

#[cfg(not(any(feature = "rusqlite", feature = "postgres")))]
compile_error!("Enable one of the following features: sqlite, pg");

#[cfg(all(feature = "rusqlite", feature = "postgres"))]
compile_error!("Only one of the following features must be enabled: sqlite, pg");

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
extern crate rand;
#[macro_use]
extern crate syn;

#[macro_use]
mod hashmap;
mod analyzer;
mod arguments;
mod ast;
mod attribute;
mod error;
mod gen;
mod methods;
mod optimizer;
mod parser;
mod plugin;
mod sql;
mod stable;
mod state;
mod string;
mod types;

use std::iter::FromIterator;

use proc_macro::TokenStream;
#[cfg(feature = "unstable")]
use proc_macro::{TokenNode, TokenTree};
use proc_macro2::{Spacing, Span};
use quote::Tokens;
#[cfg(feature = "unstable")]
use quote::ToTokens;
use syn::{
    Expr,
    Ident,
    Item,
    ItemEnum,
    parse,
    parse2,
};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use analyzer::{
    analyze,
    analyze_methods,
    analyze_types,
    get_aggregate_calls,
    get_insert_idents,
    get_limit_args,
    get_method_calls,
    get_sort_idents,
    get_values_idents,
};
#[cfg(feature = "unstable")]
use analyzer::get_insert_position;
use arguments::{Arg, Args, arguments};
use ast::{
    Aggregate,
    Expression,
    Join,
    MethodCall,
    Query,
    QueryType,
    query_type,
};
use error::{Error, Result};
#[cfg(not(feature = "unstable"))]
use error::compiler_error;
use gen::{
    gen_check_missing_fields,
    generate_errors,
    gen_query,
    get_struct_fields,
    table_macro,
    table_methods,
    tosql_impl,
};
use optimizer::optimize;
use parser::Parser;
use stable::generate_macro_patterns;

struct SqlQueryWithArgs {
    aggregate_calls: Vec<(String, Expr)>,
    aggregates: Vec<Aggregate>,
    arguments: Args,
    filter_method_calls: Vec<(MethodCall, Option<Expression>)>,
    idents: Vec<Ident>,
    #[cfg(feature = "unstable")]
    insert_call_span: Option<Span>,
    insert_idents: Option<Vec<Ident>>,
    joins: Vec<Join>,
    limit_exprs: Vec<Expr>,
    literal_arguments: Args,
    query_type: QueryType,
    sql: Tokens,
    stable_macro_query: Tokens,
    table_name: Ident,
}

/// Expand the `sql!()` macro.
/// This macro converts the Rust code provided as argument to SQL and outputs Rust code using the
/// `postgres` library.
#[cfg(feature = "unstable")]
#[proc_macro]
pub fn sql(input: TokenStream) -> TokenStream {
    let arguments: Arguments =
        match parse(input) {
            Ok(args) => args,
            Err(error) => return generate_errors(vec![Error::new(
                    &format!("cannot parse expression in sql!(): {}", error), Span::call_site())]),
        };
    let arguments = arguments.0;
    let (sql_expr, connection_ident) =
        match arguments.len() {
            2 => {
                let connection_expr = &arguments[0];
                let connection_expr = quote! {
                    #connection_expr
                };
                (&arguments[1], connection_expr)
            }
            1 => (&arguments[0], default_connection_expr()),
            arg_count =>
                return generate_errors(vec![Error::new_with_code(
                        &format!("this macro takes 1 parameter but {} parameters were supplied", arg_count),
                                                     Span::call_site(), "E0061")]),
        };
    let sql_expr = quote! { #sql_expr };
    let sql_result = to_sql_query(sql_expr.into());
    match sql_result {
        Ok(sql_query_with_args) => gen_query(&sql_query_with_args, connection_ident).0,
        Err(errors) => generate_errors(errors),
    }
}

/// Expand the `to_sql!()` macro.
/// This macro converts the Rust code provided as argument to SQL and ouputs it as a string
/// expression.
#[cfg(feature = "unstable")]
#[proc_macro]
pub fn to_sql(input: TokenStream) -> TokenStream {
    match to_sql_query(input.into()) {
        Ok(args) => args.sql.into(),
        Err(errors) => generate_errors(errors),
    }
}

/// Convert the Rust code to an SQL string with its type, arguments, joins, and aggregate fields.
fn to_sql_query(input: proc_macro2::TokenStream) -> Result<SqlQueryWithArgs> {
    if input.is_empty() {
        return Err(vec![Error::new_with_code("this macro takes 1 parameter but 0 parameters were supplied",
                                             input.span(), "E0061")]);
    }
    let expr: Expr =
        match parse2(input) {
            Ok(expr) => expr,
            Err(error) => return Err(vec![Error::new(&error.to_string(), Span::call_site())]),
        };
    let parser = Parser::new();
    let method_calls = parser.parse(&expr)?;
    let table_name = method_calls.name.clone().expect("table name in method_calls");
    #[cfg(feature = "unstable")]
    let insert_call_span = get_insert_position(&method_calls);
    let mut query = analyze(&method_calls)?;
    analyze_methods(&query)?;
    optimize(&mut query);
    analyze_types(&query)?;
    let sql = query.to_tokens();
    let joins =
        match query {
            Query::Select { ref joins, .. } => joins.clone(),
            _ => vec![],
        };
    let aggregates: Vec<Aggregate> =
        match query {
            Query::Aggregate { ref aggregates, .. } => aggregates.clone(),
            _ => vec![],
        };
    let query_type = query_type(&query);
    let mut idents = get_sort_idents(&query);
    idents.extend(get_values_idents(&query));
    let insert_idents = get_insert_idents(&query);
    let limit_exprs = get_limit_args(&query);
    let filter_method_calls = get_method_calls(&query);
    let aggregate_calls = get_aggregate_calls(&query);
    let stable_macro_query = generate_macro_patterns(&query, &method_calls);
    let (arguments, literal_arguments) = arguments(query);
    Ok(SqlQueryWithArgs {
        aggregates,
        aggregate_calls,
        arguments,
        filter_method_calls,
        idents,
        #[cfg(feature = "unstable")]
        insert_call_span,
        insert_idents,
        joins,
        limit_exprs,
        literal_arguments,
        query_type,
        sql,
        stable_macro_query,
        table_name,
    })
}

/// Expand the `#[SqlTable]` attribute.
/// This attribute must be used on structs to tell tql that it represents an SQL table.
#[proc_macro_derive(SqlTable)]
pub fn sql_table(input: TokenStream) -> TokenStream {
    let item: Item =
        match parse(input) {
            Ok(item) => item,
            Err(error) => return generate_errors(vec![Error::new(
                    &format!("cannot parse expression in SqlTable: {}", error), Span::call_site())]),
        };

    let gen =
        if let Item::Struct(item_struct) = item {
            let (fields, primary_key, impls) = get_struct_fields(&item_struct);
            let mut compiler_errors = quote! {};
            let errors =
                if let Err(errors) = fields {
                    for error in errors {
                        add_error(error, &mut compiler_errors);
                    }
                    compiler_errors
                }
                else {
                    quote! {
                    }
                };
            let code = tosql_impl(&item_struct, primary_key);
            let methods = table_methods(&item_struct);
            let table_macro = table_macro(&item_struct);
            let code = quote! {
                #errors
                #methods
                #code
                #table_macro
            };
            concat_token_stream(code.into(), impls)
        }
        else {
            let mut compiler_errors = quote! {};
            let error = Error::new("Expected struct but found", item.span()); // TODO: improve this message.
            add_error(error, &mut compiler_errors);
            compiler_errors.into()
        };

    gen
}

#[cfg(feature = "unstable")]
fn respan_tokens_with(tokens: Tokens, span: proc_macro::Span) -> Tokens {
    let tokens: proc_macro2::TokenStream = respan_with(tokens.into(), span).into();
    tokens.into_tokens()
}

#[cfg(feature = "unstable")]
fn respan_with(tokens: TokenStream, span: proc_macro::Span) -> TokenStream {
    let mut result = vec![];
    for mut token in tokens {
        match token.kind {
            TokenNode::Group(delimiter, inner_tokens) => {
                let new_tokens = respan_with(inner_tokens, span);
                result.push(TokenTree {
                    span,
                    kind: TokenNode::Group(delimiter, new_tokens),
                });
            },
            _ => {
                token.span = span;
                result.push(token);
            }
        }
    }
    FromIterator::from_iter(result.into_iter())
}

/// Get the arguments to send to the `postgres::stmt::Statement::query` or
/// `postgres::stmt::Statement::execute` method.
fn typecheck_arguments(args: &SqlQueryWithArgs) -> (Tokens, Vec<Tokens>) {
    let table_ident = &args.table_name;
    let mut arg_refs = vec![];
    let mut fns = vec![];
    let mut assigns = vec![];
    let mut typechecks = vec![];
    #[cfg(not(feature = "unstable"))]
    let mut metavars = vec![];
    #[cfg(feature = "unstable")]
    let metavars = vec![];
    let mut next_name = (0..).map(|counter|
        Ident::from(format!("__tql_arg{}", counter))
    );

    let ident = Ident::from("_table");
    {
        let mut add_arg = |arg: &Arg| {
            let arg_name =
                match arg.expression {
                    Expr::Lit(_) => None,
                    _ => Some(next_name.next().expect("Next name")),
                };
            if let Some(name) = arg.field_name.as_ref()
                .map(|name| {
                    let pos = name.span();
                    let name = name.to_string();
                    let index = name.find('.')
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    Ident::new(&name[index..], pos)
                })
            {
                let convert_ident = Ident::new("convert", arg.expression.span());
                let to_owned_ident = Ident::new("to_owned", Span::call_site());
                #[cfg(not(feature = "unstable"))]
                let expr = arg_name.map(|arg_name| quote! { #arg_name }).unwrap_or_else(|| {
                    let expr = &arg.expression;
                    quote! { #expr }
                });
                #[cfg(feature = "unstable")]
                let expr = &arg.expression;
                assigns.push(quote_spanned! { arg.expression.span() =>
                    #ident.#name = #convert_ident(&#expr.#to_owned_ident());
                });
                fns.push(quote_spanned! { arg.expression.span() =>
                    // NOTE: hack to get the type required by the field struct.
                    fn #convert_ident<T: ::std::ops::Deref>(_arg: T) -> T::Target
                    where T::Target: Sized
                    {
                        unimplemented!()
                    }
                });
            }
            arg_name
        };

        for arg in &args.arguments {
            let _name = add_arg(&arg);
            match arg.expression {
                // Do not add literal arguments as they are in the final string literal.
                Expr::Lit(_) => (),
                _ => {
                    #[cfg(not(feature = "unstable"))]
                    {
                        if let Some(name) = _name {
                            metavars.push(quote! { #name });
                            arg_refs.push(quote! { &#name })
                        }
                        else {
                            let expr = &arg.expression;
                            arg_refs.push(quote! { &(#expr) });
                        }
                    }
                    #[cfg(feature = "unstable")]
                    {
                        let expr = &arg.expression;
                        arg_refs.push(quote! { &(#expr) });
                    }
                },
            }
        }

        for arg in &args.literal_arguments {
            add_arg(&arg);
        }
    }

    for name in &args.idents {
        typechecks.push(quote_spanned! { name.span() =>
            #ident.#name = unsafe { ::std::mem::zeroed() };
        });
    }

    for expr in &args.limit_exprs {
        typechecks.push(quote! {{
            let _: i64 = #expr;
        }});
    }

    let macro_name = Ident::new(&format!("tql_{}_check_missing_fields", table_ident), Span::call_site());
    if let Some(ref insert_idents) = args.insert_idents {
        let code = quote! {
            #macro_name!(#(#insert_idents),*);
        };
        #[cfg(feature = "unstable")]
        let code = {
            let span = args.insert_call_span.expect("insert() span");
            respan_tokens_with(code, span.unstable())
        };
        typechecks.push(code);
    }

    for data in &args.filter_method_calls {
        let call = &data.0;
        let field = &call.object_name;
        let method = &call.method_name;
        let arguments = &call.arguments;
        let trait_ident = quote_spanned! { table_ident.span() =>
            tql::ToTqlType;
        };
        let method_name = quote_spanned! { table_ident.span() =>
            to_tql_type
        };
        let comparison_expr =
            if let Some(ref expr) = data.1 {
                quote! {
                    let mut _data = #field.#method(#(#arguments),*);
                    _data = #expr;
                }
            }
            else {
                quote_spanned! { call.position =>
                    true == #field.#method(#(#arguments),*);
                }
            };
        typechecks.push(quote! {{
            use #trait_ident;
            let #field = #ident.#field.#method_name();
            #comparison_expr
        }});
    }

    let tql_ident = quote_spanned! { Span::call_site() =>
        ::tql
    };
    for &(ref function, ref expr) in &args.aggregate_calls {
        let function = Ident::new(function, Span::call_site());
        typechecks.push(quote! {
            let mut _data = #tql_ident::aggregates::#function();
            _data = #expr;
        });
    }

    let trait_ident = quote_spanned! { table_ident.span() =>
        ::tql::SqlTable
    };

    let tokens = quote_spanned! { table_ident.span() => {
        // Type check the arguments by creating a dummy struct.
        // TODO: check that this let is not in the generated binary.
        {
            let _tql_closure = || {
                let mut #ident = <#table_ident as #trait_ident>::_tql_default();
                #({
                    #fns
                    #assigns
                })*
                #(#typechecks)*
            };
        }

        [#(#arg_refs),*]
    }};
    (tokens, metavars)
}

fn concat_token_stream(stream1: TokenStream, stream2: TokenStream) -> TokenStream {
    FromIterator::from_iter(stream1.into_iter().chain(stream2.into_iter()))
}

// TODO: replace by TokenStream::empty() when stable.
fn empty_token_stream() -> TokenStream {
    (quote! {}).into()
}

#[cfg(feature = "unstable")]
fn add_error(error: Error, _compiler_errors: &mut Tokens) {
    error.emit_diagnostic();
}

#[cfg(not(feature = "unstable"))]
fn add_error(error: Error, compiler_errors: &mut Tokens) {
    let error = compiler_error(&error);
    let old_errors = compiler_errors.clone();
    *compiler_errors = quote! {
        #old_errors
        #error
    };
}

#[cfg(feature = "unstable")]
#[proc_macro]
pub fn check_missing_fields(input: TokenStream) -> TokenStream {
    gen_check_missing_fields(input)
}

struct Arguments(Punctuated<Expr, Token![,]>);

impl syn::synom::Synom for Arguments {
    named!(parse -> Self, map!(call!(Punctuated::parse_terminated), Arguments));
}

fn default_connection_expr() -> Tokens {
    let connection_ident = Ident::new("connection", Span::call_site());
    quote! {
        #connection_ident
    }
}

// Stable implementation.

#[proc_macro_derive(StableCheckMissingFields)]
pub fn stable_check_missing_fieds(input: TokenStream) -> TokenStream {
    let enumeration: Item = parse(input).unwrap();
    if let Item::Enum(ItemEnum { ref variants, .. }) = enumeration {
        let variant = &variants.first().unwrap().value().discriminant;
        if let Expr::Field(ref field) = variant.as_ref().unwrap().1 {
            if let Expr::Tuple(ref tuple) = *field.base {
                if let Expr::Macro(ref macr) = **tuple.elems.first().unwrap().value() {
                    let code = gen_check_missing_fields(macr.mac.tts.clone().into());
                    let code = proc_macro2::TokenStream::from(code);

                    let gen = quote! {
                        macro_rules! __tql_call_macro_missing_fields {
                            () => {{
                                #code
                            }};
                        }
                    };
                    return gen.into();
                }
            }
        }
    }

    empty_token_stream()
}

// TODO: make this function more robust.
#[proc_macro_derive(StableToSql)]
pub fn stable_to_sql(input: TokenStream) -> TokenStream {
    let enumeration: Item = parse(input).unwrap();
    if let Item::Enum(ItemEnum { ref variants, .. }) = enumeration {
        let variant = &variants.first().unwrap().value().discriminant;
        if let Expr::Field(ref field) = variant.as_ref().unwrap().1 {
            if let Expr::Tuple(ref tuple) = *field.base {
                if let Expr::Macro(ref macr) = **tuple.elems.first().unwrap().value() {
                    let tts: Vec<_> = macr.mac.tts.clone().into_iter().collect();
                    let (sql_query, connection_expr) =
                        if let proc_macro2::TokenNode::Op(',', Spacing::Alone) = tts[1].kind {
                            let connection_expr = &tts[0];
                            let connection_expr = quote! {
                                #connection_expr
                            };
                            (&tts[2..], connection_expr)
                        }
                        else {
                            (&tts[..], default_connection_expr())
                        };
                    let sql_query = sql_query.iter()
                        .map(|tt| proc_macro2::TokenStream::from(tt.clone()));
                    let sql_query = proc_macro2::TokenStream::from_iter(sql_query);
                    let sql_result = to_sql_query(sql_query);
                    let (code, metavars, extract_macro) = match sql_result {
                        Ok(sql_query_with_args) => {
                            let (code, metavars) = gen_query(&sql_query_with_args, connection_expr);
                            (code, metavars, sql_query_with_args.stable_macro_query)
                        },
                        Err(errors) => (generate_errors(errors), vec![], quote! {}),
                    };
                    let code = proc_macro2::TokenStream::from(code);
                    let vars =
                        match metavars.len() {
                            0 => quote! {},
                            1 => quote! {
                                let ref #(#metavars),* = __tql_extract_exprs!($($tt)*);
                            },
                            _ => quote! {
                                let (ref #(#metavars),*) = __tql_extract_exprs!($($tt)*);
                            }
                        };

                    let gen = quote! {
                        #extract_macro

                        macro_rules! __tql_call_macro {
                            ($connection:ident, $($tt:tt)*) => {{
                                let ref connection = $connection;
                                #vars
                                #code
                            }};
                        }
                    };
                    return gen.into();
                }
            }
        }
    }

    empty_token_stream()
}
