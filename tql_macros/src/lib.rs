/*
 * Primary key field
 * SQLite: ROWID
 *
 * TODO: test that get() does not work when the primary key is not named id.
 * TODO: support recursive foreign key.
 * TODO: use fully-qualified name everywhere in the query (aggregate, …).
 * TODO: check errors for joined tables.
 * TODO: allow selecting only some fields.
 * TODO: remove allow_failure for beta when this issue is fixed:
 * https://github.com/rust-lang/rust/issues/46478
 * TODO: for the tests of the other backend, create a new crate and include!() the _expr test files
 * and create a new test to check that all the files are included, so that the tests fail when we
 * forget to include!() a file.
 * TODO: write fail tests for stable using include!().
 * TODO: remove the internal state of the proc-macro and use dummy code generation to check the
 * identifiers (to make it work with models defined in external crates). Also, use the trait bound
 * SqlTable trick to check that it is a table.
 * TODO: allow using other fields in filter(), update(), … like F() expressions in Django
 ** Table.filter(field1 > Table.field2) may not work.
 ** Table.filter(field1 > $field2)
 * TODO: ManyToMany.
 * TODO: support other types (uuid, string) for the primary key, possibly by making it generic.
 * TODO: support the missing types
 * (https://docs.rs/postgres/0.15.1/postgres/types/trait.ToSql.html).
 * TODO: join on non foreign key.
 * TODO: unique constraints.
 * TODO: support primary key with multiple columns.
 * TODO: allow user-defined functions (maybe with partial query?) and types.
 * TODO: document the management of the connection.
 * TODO: add table_name attribute to allow changing the table name.
 * TODO: improve the error handling of the generated code.
 * TODO: use as_ref() for Ident instead of &ident.to_string().
 * TODO: improve formatting of the README table.
 * TODO: the error message sometimes show String instead of &str.
 * FIXME: warning should not be errors on stable.
 *
 * TODO: switch to a binding to a C postgresql library for better performance?
 * FIXME: postgres crate might be using dynamic dispatch (ToSql), we might get better performance
 * if we avoid this.
 */

#![cfg_attr(feature = "unstable", feature(proc_macro))]
#![recursion_limit="128"]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
extern crate rand;
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
mod state;
mod string;
mod types;

use std::collections::BTreeMap;
use std::iter::FromIterator;

use proc_macro::TokenStream;
#[cfg(feature = "unstable")]
use proc_macro::{TokenNode, TokenTree};
use proc_macro2::Span;
use quote::Tokens;
#[cfg(feature = "unstable")]
use quote::ToTokens;
use rand::Rng;
use syn::{
    AngleBracketedGenericArguments,
    Expr,
    Field,
    Fields,
    FieldsNamed,
    GenericArgument,
    Ident,
    Item,
    ItemEnum,
    ItemStruct,
    TypePath,
    parse,
    parse2,
};
#[cfg(feature = "unstable")]
use syn::{LitStr, Path};
use syn::PathArguments::AngleBracketed;
use syn::spanned::Spanned;

use analyzer::{analyze, analyze_types, has_joins};
use arguments::{Args, arguments};
use ast::{
    Aggregate,
    Join,
    Query,
    QueryType,
    TypedField,
    query_type,
};
use attribute::{field_ty_to_type, fields_vec_to_hashmap};
use error::{Error, Result, res};
#[cfg(not(feature = "unstable"))]
use error::compiler_error;
use gen::ToSql;
use optimizer::optimize;
use parser::Parser;
use plugin::string_literal;
use state::SqlFields;
use string::token_to_string;
use types::Type;

struct SqlQueryWithArgs {
    aggregates: Vec<Aggregate>,
    arguments: Args,
    joins: Vec<Join>,
    query_type: QueryType,
    #[cfg(feature = "unstable")]
    span: Span,
    sql: String,
    table_name: Ident,
}

/// Expand the `sql!()` macro.
/// This macro converts the Rust code provided as argument to SQL and outputs Rust code using the
/// `postgres` library.
#[cfg(feature = "unstable")]
#[proc_macro]
pub fn sql(input: TokenStream) -> TokenStream {
    // TODO: if the first parameter is not provided, use "connection".
    // TODO: to do so, try to parse() to a Punctuated(Comma, syn::Expr).
    let sql_result = to_sql_query(input.into());
    match sql_result {
        Ok(sql_query_with_args) => gen_query(sql_query_with_args),
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
        Ok(args) => {
            let gen =
                match args.query_type {
                    QueryType::Create => {
                        let table_name = args.table_name;
                        quote! {
                            #table_name::_create_query()
                        }
                    },
                    _ => {
                        let expr = LitStr::new(&args.sql, args.span);
                        quote! {
                            #expr
                        }
                    }
                };
            gen.into()
        },
        Err(errors) => generate_errors(errors),
    }
}

/// Convert the Rust code to an SQL string with its type, arguments, joins, and aggregate fields.
fn to_sql_query(input: proc_macro2::TokenStream) -> Result<SqlQueryWithArgs> {
    // TODO: use this when it becomes stable.
    /*if input.is_empty() {
        return Err(vec![Error::new_with_code("this macro takes 1 parameter but 0 parameters were supplied", cx.call_site(), "E0061")]);
    }*/
    let expr: Expr =
        match parse2(input) {
            Ok(expr) => expr,
            Err(error) => return Err(vec![Error::new(&error.to_string(), Span::default())]),
        };
    #[cfg(feature = "unstable")]
    let span = expr.span();
    let parser = Parser::new();
    let method_calls = parser.parse(&expr)?;
    let table_name = method_calls.name.clone().expect("table name in method_calls");
    let mut query = analyze(method_calls)?;
    optimize(&mut query);
    query = analyze_types(query)?;
    let sql = query.to_sql();
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
    let arguments = arguments(query);
    Ok(SqlQueryWithArgs {
        aggregates,
        arguments,
        joins,
        query_type,
        #[cfg(feature = "unstable")]
        span,
        sql,
        table_name,
    })
}

/// Expand the `#[SqlTable]` attribute.
/// This attribute must be used on structs to tell tql that it represents an SQL table.
#[proc_macro_derive(SqlTable)]
pub fn sql_table(input: TokenStream) -> TokenStream {
    let item: Item = parse(input).expect("parse expression in sql_table()");

    let gen =
        if let Item::Struct(item_struct) = item {
            let table_name = item_struct.ident.to_string();
            let (fields, primary_key, impls) = get_struct_fields(&item_struct);
            let mut compiler_errors = quote! {};
            if let Ok(fields) = fields {
                // NOTE: Transform the span by dummy spans to workaround this issue:
                // https://github.com/rust-lang/rust/issues/42337
                // https://github.com/rust-lang/rust/issues/45934#issuecomment-344497531
                // NOTE: if there is no error, there is a primary key, hence expect().
                let code = tosql_impl(&item_struct, &primary_key.expect("primary key"));
                let new_structs = create_typecheck_structs(&item_struct);
                let methods = table_methods(&item_struct);
                let code = quote! {
                    #code
                    #new_structs
                    #methods
                }.into();
                #[cfg(feature = "unstable")]
                let code = respan(code);
                return concat_token_stream(code, impls);
            }
            if let Err(errors) = fields {
                for error in errors {
                    add_error(error, &mut compiler_errors);
                }
            }
            concat_token_stream(compiler_errors.into(), impls)
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
fn respan(tokens: TokenStream) -> TokenStream {
    respan_with(tokens, proc_macro::Span::call_site())
}

#[cfg(feature = "unstable")]
fn respan_tokens(tokens: Tokens) -> Tokens {
    let tokens: proc_macro2::TokenStream = respan(tokens.into()).into();
    tokens.into_tokens()
}

#[cfg(not(feature = "unstable"))]
fn respan_tokens(tokens: Tokens) -> Tokens {
    tokens
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

/// Get the fields from the struct (also returns the ToSql implementations to check that the types
/// used for ForeignKey have a #[derive(SqlTable)]).
/// Also check if the field types from the struct are supported types.
fn get_struct_fields(item_struct: &ItemStruct) -> (Result<SqlFields>, Option<String>, TokenStream) {
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
                                let segment = segments.first().expect("first segment").into_item();
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

/// Create the structures used to type check the queries.
fn create_typecheck_structs(item_struct: &ItemStruct) -> Tokens {
    let table_ident = &item_struct.ident;
    if let Fields::Named(FieldsNamed { ref named , .. }) = item_struct.fields {
        let field_idents = named.iter().map(|field| &field.ident);
        let field_idents2 = named.iter().map(|field| &field.ident);

        let mut string_found = false;
        let field_types: Vec<_> =
            named.iter()
                .map(|field| {
                    if token_to_string(&field.ty) == "String" {
                        string_found = true;
                        quote! {
                            &'a str
                        }
                    }
                    else {
                        let ty = &field.ty;
                        quote! {
                            #ty
                        }
                    }
                })
                .collect();
        let module_name = Ident::new(&format!("__tql_{}", rand_string().to_lowercase()), Span::default());
        let lifetime =
            if string_found {
                quote! {
                    <'a>
                }
            }
            else {
                quote! {
                }
            };
        quote! {
            mod #module_name {
                struct #table_ident#lifetime {
                    #(#field_idents: #field_types,)*
                }

                impl#lifetime Default for #table_ident#lifetime {
                    #[inline(always)]
                    fn default() -> Self {
                        #table_ident {
                            #(#field_idents2: unsafe { ::std::mem::zeroed() },)*
                        }
                    }
                }
            }
        }
    }
    else {
        unreachable!("Check is done in get_struct_fields()")
    }
}

/// Create the _create_query() and from_row() method for the table struct.
fn table_methods(item_struct: &ItemStruct) -> Tokens {
    let table_ident = &item_struct.ident;
    if let Fields::Named(FieldsNamed { ref named , .. }) = item_struct.fields {
        let mut fields_to_create = vec![];
        for field in named {
            fields_to_create.push(TypedField {
                identifier: field.ident.expect("field ident").to_string(),
                typ: field_ty_to_type(&field.ty).node.to_sql(),
            });
        }
        let create_query = format!("CREATE TABLE {table} ({fields})",
            table = table_ident,
            fields = fields_to_create.to_sql()
        );

        let field_names = named.iter()
            .map(|field| field.ident.expect("field has name"));
        let field_names2 = named.iter()
            .map(|field| field.ident.expect("field has name"));

        let field_idents = named.iter()
            .map(|field| (field.ident.expect("field has name"), field.ty.clone()));
        let columns = field_idents.map(|(ident, typ)| to_row_get(&ident, typ, ""));

        let field_idents = named.iter()
            .map(|field| (field.ident.expect("field has name"), field.ty.clone()));
        let fully_qualified_columns = field_idents.map(|(ident, typ)|
            to_row_get(&ident, typ, &format!("{}.", table_ident)));

        quote! {
            impl #table_ident {
                pub fn _create_query() -> &'static str {
                    #create_query
                }

                #[allow(unused)]
                pub fn from_row(row: &::postgres::rows::Row) -> Self {
                    Self {
                        #(#field_names: #columns,)*
                    }
                }

                #[allow(unused)]
                pub fn from_joined_row(row: &::postgres::rows::Row) -> Self {
                    Self {
                        #(#field_names2: #fully_qualified_columns,)*
                    }
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
fn tosql_impl(item_struct: &ItemStruct, primary_key_field: &str) -> Tokens {
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

        unsafe impl ::tql::SqlTable for #table_ident {
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

fn generate_errors(errors: Vec<Error>) -> TokenStream {
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
        let expr = LitStr::new("", Span::default());
        let gen = quote! {
            #expr
        };
        gen.into()
    }
}

/// Generate the Rust code from the SQL query.
fn gen_query(args: SqlQueryWithArgs) -> TokenStream {
    let table_ident = &args.table_name;
    let ident = Ident::new("connection", table_ident.span);
    let struct_expr = create_struct(table_ident, args.joins);
    let (aggregate_struct, aggregate_expr) = gen_aggregate_struct(&args.aggregates);
    let args_expr = get_query_arguments(args.arguments);
    let tokens = gen_query_expr(ident, args.sql, args_expr, struct_expr, aggregate_struct, aggregate_expr,
                                args.query_type, table_ident);
    tokens.into()
}

/// Generate the Rust code using the `postgres` library depending on the `QueryType`.
fn gen_query_expr(connection_ident: Ident, sql_query: String, args_expr: Tokens, struct_expr: Tokens,
                  aggregate_struct: Tokens, aggregate_expr: Tokens, query_type: QueryType, table_ident: &Ident) -> Tokens
{
    let sql_query = string_literal(&sql_query);
    match query_type {
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
                #connection_ident.prepare(#table_ident::_create_query())
                    .and_then(|result| result.execute(&[]))
            }}
        },
        QueryType::InsertOne => {
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
            let result =
                quote! {{
                    let result = #connection_ident.prepare(#sql_query).expect("prepare query");
                    result.query(&#args_expr).expect("execute query").iter()
                }};
            let call = respan_tokens(quote! {
                .map(|row| {
                    #struct_expr
                }).collect::<Vec<_>>()
                // TODO: return an iterator instead of a vector.

            });
            quote! {
                #result#call
            }
        },
        QueryType::SelectOne => {
            let result =
                quote! {{
                    let result = #connection_ident.prepare(#sql_query).expect("prepare query");
                    result.query(&#args_expr).expect("execute query").iter().next()
                }};
            let call = respan_tokens(quote! {
                .map(|row| {
                    #struct_expr
                })
            });
            quote! {
                #result#call
            }
        },
        QueryType::Exec => {
            quote! {{
                #connection_ident.prepare(#sql_query)
                    .and_then(|result| result.execute(&#args_expr))
            }}
        },
    }
}

/// Get the arguments to send to the `postgres::stmt::Statement::query` or
/// `postgres::stmt::Statement::execute` method.
fn get_query_arguments(arguments: Args) -> Tokens {
    let mut arg_refs = vec![];
    let mut qualifiers = vec![];
    let mut idents = vec![];
    let mut types: Vec<String> = vec![];
    let mut exprs = vec![];

    for arg in arguments {
        match arg.expression {
            // Do not add literal arguments as they are in the final string literal.
            Expr::Lit(_) => (),
            _ => {
                let expr = &arg.expression;
                arg_refs.push(quote! { &(#expr) });
            },
        }

        let name = arg.field_name
            .map(|name| name.replace('.', ""))
            .unwrap_or_else(|| rand_string().to_lowercase());
        idents.push(new_ident(&format!("__tql_{}", name)));

        let exp = &arg.expression;
        exprs.push(quote! { #exp });

        // TODO
        /*if is_string_type(&arg.typ) {
            qualifiers.push(quote! {});
        }
        else {*/
            qualifiers.push(quote! { ref });
        //}

        /*if let Some(ty) = inner_type(&arg.typ) {
            types.push(quote! { #ty });
        }
        else if is_string_type(&arg.typ) {
            types.push(quote! { &str });
        }
        else {
            let ty = &arg.typ;
            types.push(quote! { #ty });
        }*/
    }

    quote! {{
        // Add the arguments as let statements so that they can be type-checked.
        // TODO: check that this let is not in the generated binary.
        #(let #qualifiers #idents: #types = #exprs;)*
        [#(#arg_refs),*]
    }}
}

/// Create the struct expression needed by the generated code.
fn create_struct(table_ident: &Ident, joins: Vec<Join>) -> Tokens {
    let mut field_idents: Vec<String> = vec![];
    let mut field_values: Vec<String> = vec![];
    let mut index = 0usize;
    let joined_fields =
        joins.iter()
            .map(|join| Ident::new(&join.base_field, Span::default()));
    let joined_tables =
        joins.iter()
            .map(|join| Ident::new(&join.joined_table, Span::default()));
    let code = quote! {{
        #[allow(unused_mut)]
        let mut item = #table_ident::from_row(&row);
        #(item.#joined_fields = Some(#joined_tables::from_joined_row(&row));)*
        item
    }};
    // TODO: when the private field issue is fixed, remove the call to respan.
    respan_tokens(code.into())
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

fn new_ident(string: &str) -> Ident {
    Ident::new(string, Span::default())
}

fn is_string_type(typ: &syn::Type) -> bool {
    if let syn::Type::Path(TypePath { ref path, .. }) = *typ {
        let element = path.segments.first().expect("first segment of path");
        let segment = element.item();
        return segment.ident.as_ref() == "String";
    }
    false
}

fn inner_type(typ: &syn::Type) -> Option<syn::Type> {
    if let syn::Type::Path(TypePath { ref path, .. }) = *typ {
        let element = path.segments.first().expect("first segment of path");
        let segment = element.item();
        match segment.ident.as_ref() {
            "Option" | "ForeignKey" => {
                if let AngleBracketed(AngleBracketedGenericArguments { ref args, .. }) = segment.arguments {
                    let element = args.first().expect("first arg of args");
                    let arg = element.item();
                    if let GenericArgument::Type(ref ty) = **arg {
                        return Some(ty.clone());
                    }
                }
            },
            _ => (),
        }
    }
    None
}

fn rand_string() -> String {
    rand::thread_rng().gen_ascii_chars().take(30).collect()
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

fn to_row_get(ident: &Ident, typ: syn::Type, ident_prefix: &str) -> Tokens {
    if let syn::Type::Path(TypePath { path: Path { ref segments, .. }, .. }) = typ {
        let segment = segments.first().expect("first segment").into_item();
        if segment.ident == "ForeignKey" {
            return quote! {
                None
            };
        }
    }
    let field_name = format!("{}{}", ident_prefix, ident);
    quote! {
        row.get(#field_name)
    }
}

// Stable implementation.

// TODO: make this function more robust.
#[proc_macro_derive(StableToSql)]
pub fn stable_to_sql(input: TokenStream) -> TokenStream {
    let enumeration: Item = parse(input).unwrap();
    if let Item::Enum(ItemEnum { ref variants, .. }) = enumeration {
        let variant = &variants.first().unwrap().item().discriminant;
        if let Expr::Field(ref field) = variant.as_ref().unwrap().1 {
            if let Expr::Tuple(ref tuple) = *field.base {
                if let Expr::Macro(ref macr) = **tuple.elems.first().unwrap().item() {
                    let sql_result = to_sql_query(macr.mac.tts.clone());
                    let code = match sql_result {
                        Ok(sql_query_with_args) => gen_query(sql_query_with_args),
                        Err(errors) => generate_errors(errors),
                    };
                    let code = proc_macro2::TokenStream::from(code);

                    let gen = quote! {
                        macro_rules! __tql_call_macro {
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
