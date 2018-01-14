/*
 * Primary key field
 * SQLite: ROWID
 *
 * FIXME: update all generated identifiers to avoid name clash.
 * TODO: don't hard-code "id" for join.
 *
 * TODO: try to get the table of a foreign key field so that it's not necessary to specify in the
 * query.
 * TODO: document the management of the connection.
 * TODO: improve the error handling of the generated code.
 * TODO: test that get() does not work when the primary key is not named id.
 * TODO: use as_ref() for Ident instead of &ident.to_string().
 * TODO: support recursive foreign key.
 * TODO: write fail tests for stable using include!().
 * TODO: try to get the columns by OID from postgres to improve the syntax.
 * TODO: try to hide Option in the mismatched type error message for ForeignKey.
 * TODO: use fully-qualified name everywhere in the query (aggregate, …).
 * TODO: check errors for joined tables.
 * TODO: for the tests of the other backend, create a new crate and include!() the _expr test files
 * and create a new test to check that all the files are included, so that the tests fail when we
 * forget to include!() a file.
 *
 * TODO: ManyToMany.
 * TODO: support the missing types
 * (https://docs.rs/postgres/0.15.1/postgres/types/trait.ToSql.html).
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
mod state;
mod string;
mod types;

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
    Expr,
    Field,
    Fields,
    FieldsNamed,
    Ident,
    Item,
    ItemEnum,
    ItemStruct,
    parse,
    parse2,
};
#[cfg(feature = "unstable")]
use syn::{AngleBracketedGenericArguments, LitStr, Path, TypePath};
#[cfg(feature = "unstable")]
use syn::PathArguments::AngleBracketed;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use analyzer::{
    analyze,
    analyze_types,
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
use types::{Type, get_type_parameter};

struct SqlQueryWithArgs {
    aggregates: Vec<Aggregate>,
    arguments: Args,
    idents: Vec<Ident>,
    #[cfg(feature = "unstable")]
    insert_call_span: Option<Span>,
    insert_idents: Option<Vec<Ident>>,
    joins: Vec<Join>,
    limit_exprs: Vec<Expr>,
    literal_arguments: Args,
    method_calls: Vec<(MethodCall, Option<Expression>)>,
    query_type: QueryType,
    #[cfg(feature = "unstable")]
    span: Span,
    sql: String,
    table_name: Ident,
    use_pk: bool,
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
                        let trait_ident = quote_spanned! { args.table_name.span() =>
                            ::tql::SqlTable
                        };

                        let table_name = args.table_name;
                        quote! {
                            <#table_name as #trait_ident>::_create_query()
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
            Err(error) => return Err(vec![Error::new(&error.to_string(), Span::call_site())]),
        };
    #[cfg(feature = "unstable")]
    let span = expr.span();
    let parser = Parser::new();
    let method_calls = parser.parse(&expr)?;
    let table_name = method_calls.name.clone().expect("table name in method_calls");
    #[cfg(feature = "unstable")]
    let insert_call_span = get_insert_position(&method_calls);
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
    let mut idents = get_sort_idents(&query);
    idents.extend(get_values_idents(&query));
    let insert_idents = get_insert_idents(&query);
    let limit_exprs = get_limit_args(&query);
    let method_calls = get_method_calls(&query);
    let use_pk = get_use_pk(&query);
    let (arguments, literal_arguments) = arguments(query);
    Ok(SqlQueryWithArgs {
        aggregates,
        arguments,
        idents,
        #[cfg(feature = "unstable")]
        insert_call_span,
        insert_idents,
        joins,
        limit_exprs,
        literal_arguments,
        method_calls,
        query_type,
        #[cfg(feature = "unstable")]
        span,
        sql,
        table_name,
        use_pk,
    })
}

/// Expand the `#[SqlTable]` attribute.
/// This attribute must be used on structs to tell tql that it represents an SQL table.
#[proc_macro_derive(SqlTable)]
pub fn sql_table(input: TokenStream) -> TokenStream {
    let item: Item = parse(input).expect("parse expression in sql_table()");

    let gen =
        if let Item::Struct(item_struct) = item {
            let (fields, primary_key, impls) = get_struct_fields(&item_struct);
            let mut compiler_errors = quote! {};
            if let Err(errors) = fields {
                for error in errors {
                    add_error(error, &mut compiler_errors);
                }
                concat_token_stream(compiler_errors.into(), impls)
            }
            else {
                // NOTE: Transform the span by dummy spans to workaround this issue:
                // https://github.com/rust-lang/rust/issues/42337
                // https://github.com/rust-lang/rust/issues/45934#issuecomment-344497531
                // NOTE: if there is no error, there is a primary key, hence expect().
                let code = tosql_impl(&item_struct, &primary_key.expect("primary key"));
                let methods = table_methods(&item_struct);
                let table_macro = table_macro(&item_struct);
                let code = quote! {
                    #methods
                    #code
                };
                #[cfg(feature = "unstable")]
                let code = respan_tokens(code);
                let code = quote! {
                    #code
                    #table_macro
                };
                concat_token_stream(code.into(), impls)
            }
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

/// Create the _create_query() and from_row() method for the table struct.
fn table_methods(item_struct: &ItemStruct) -> Tokens {
    let table_ident = &item_struct.ident;
    if let Fields::Named(FieldsNamed { ref named , .. }) = item_struct.fields {
        let mut fields_to_create = vec![];
        let mut primary_key = None;
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
            }
        }
        let create_query = format!("CREATE TABLE {table} ({fields})",
            table = table_ident,
            fields = fields_to_create.to_sql()
        );

        let field_names = named.iter()
            .map(|field| field.ident.expect("field has name"));

        let field_idents = named.iter()
            .map(|field| (field.ident.expect("field has name"), field.ty.clone()));
        let columns = field_idents.map(|(ident, typ)| to_row_get(&table_ident, &ident, typ));

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
                fn _create_query() -> &'static str {
                    #create_query
                }

                // TODO: rename to avoid clash.
                fn default() -> Self {
                    unimplemented!()
                }

                #[allow(unused)]
                fn from_row(row: &::postgres::rows::Row, columns: &[::postgres::stmt::Column]) -> Self {
                    Self {
                        #(#field_names: #columns,)*
                    }
                }

                // TODO: move to next impl (should not be in the trait).
                fn _primary_key_field() -> &'static str {
                    #primary_key
                }
            }
        }
    }
    else {
        unreachable!("Check is done in get_struct_fields()")
    }
}

/// Create the insert macro for the table struct to check that all the mandatory fields are
/// provided.
fn table_macro(item_struct: &ItemStruct) -> Tokens {
    let table_ident = &item_struct.ident;
    let mut primary_key_found = false;
    if let Fields::Named(FieldsNamed { ref named , .. }) = item_struct.fields {
        let mut mandatory_fields = vec![];
        let mut related_table_names = vec![];
        let mut non_related_table_names = vec![];
        let mut related_table_types = vec![];
        let mut compiler_errors = vec![];
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
                        related_table_names.push(ident);
                        related_table_types.push(typ);
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
                #((#related_table_names) => { #related_table_types };)*
                #((#non_related_table_names) => { #compiler_errors };)*
                // NOTE: the check for the field name is done elsewhere, hence it is okay to return
                // "" here.
                ($tt:tt) => { "" };
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

        impl #table_ident {
            #[allow(dead_code)]
            fn to_owned(&self) -> Option<Self> {
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
        let expr = LitStr::new("", Span::call_site());
        let gen = quote! {
            #expr
        };
        gen.into()
    }
}

/// Generate the Rust code from the SQL query.
fn gen_query(args: SqlQueryWithArgs) -> TokenStream {
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
                , pk = <#table_ident as #trait_ident>::_primary_key_field()
            }
        }
        else {
            quote! { }
        };
    let joins = args.joins.iter()
        .map(|join| {
            let base_field = &join.base_field;
            let macro_name = Ident::new(&format!("tql_{}_related_tables", table_ident), Span::call_site());
            let code = quote! {
                , #base_field = #macro_name!(#base_field)
            };
            #[cfg(feature = "unstable")]
            let code = respan_tokens_with(code, base_field.span().unstable());
            code
        });
    let select_query = || {
        if args.use_pk || !args.joins.is_empty() {
            quote! {
                &format!(#sql_query #pk #(#joins)*)
            }
        }
        else {
            quote! {
                #sql_query
            }
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
                #connection_ident.prepare(<#table_ident as #trait_ident>::_create_query())
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
                    quote! {
                        &format!(#sql_query #pk)
                    }
                }
                else {
                    quote! {
                        #sql_query
                    }
                };
            quote! {{
                #connection_ident.prepare(#sql_query)
                    .and_then(|result| result.execute(&#args_expr))
            }}
        },
    }
}

/// Get the arguments to send to the `postgres::stmt::Statement::query` or
/// `postgres::stmt::Statement::execute` method.
fn typecheck_arguments(args: &SqlQueryWithArgs) -> Tokens {
    let table_ident = &args.table_name;
    let mut arg_refs = vec![];
    let mut fns = vec![];
    let mut assigns = vec![];
    let mut typechecks = vec![];

    let ident = Ident::new("_table", Span::call_site());
    {
        let mut add_arg = |arg: &Arg| {
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
                let expr = &arg.expression;
                let convert_ident = Ident::new("convert", arg.expression.span());
                assigns.push(quote_spanned! { arg.expression.span() =>
                    #ident.#name = #convert_ident(&#expr.to_owned());
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
        };

        for arg in &args.arguments {
            match arg.expression {
                // Do not add literal arguments as they are in the final string literal.
                Expr::Lit(_) => (),
                _ => {
                    let expr = &arg.expression;
                    arg_refs.push(quote! { &(#expr) });
                },
            }

            add_arg(&arg);
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

    for data in &args.method_calls {
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

    let trait_ident = quote_spanned! { table_ident.span() =>
        ::tql::SqlTable
    };

    quote_spanned! { table_ident.span() => {
        // Type check the arguments by creating a dummy struct.
        // TODO: check that this let is not in the generated binary.
        {
            let _tql_closure = || {
                let mut #ident = <#table_ident as #trait_ident>::default();
                #({
                    #fns
                    #assigns
                })*
                #(#typechecks)*
            };
        }

        [#(#arg_refs),*]
    }}
}

/// Create the struct expression needed by the generated code.
fn create_struct(table_ident: &Ident, joins: &[Join]) -> Tokens {
    let row_ident = quote! { row };
    let result_ident = Ident::from("result");
    let assign_related_fields =
        joins.iter()
            .map(|join| {
                let ident = &join.base_field;
                quote_spanned! { ident.span() => {
                    let ref mut _related_field: Option<_> = item.#ident;
                    ::tql::from_related_row(_related_field, &#row_ident, #result_ident.columns());
                }}
            });
    quote_spanned! { table_ident.span() => {
        #[allow(unused_mut)]
        let mut item = <#table_ident as ::tql::SqlTable>::from_row(&#row_ident, #result_ident.columns());
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

fn new_ident(string: &str) -> Ident {
    Ident::new(string, Span::call_site())
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

fn to_row_get(table_name: &Ident, column_name: &Ident, typ: syn::Type) -> Tokens {
    if let syn::Type::Path(path) = typ {
        let segment = path.path.segments.first().expect("first segment").into_value();
        if segment.ident == "ForeignKey" {
            return quote! {
                None
            };
        }
    }
    let table_name = table_name.to_string().to_lowercase();
    let column_name = column_name.to_string();
    quote! {
        row.get(::tql::index_from_table_column(#table_name, #column_name, columns))
    }
}

struct Arguments(Punctuated<Expr, Token![,]>);

impl syn::synom::Synom for Arguments {
    // call!(Punctuated::parse_terminated) will parse a terminated sequence of
    // Synom objects. Expr implements synom so we're good.
    named!(parse -> Self, map!(call!(Punctuated::parse_terminated), Arguments));
}

#[cfg(feature = "unstable")]
#[proc_macro]
pub fn check_missing_fields(input: TokenStream) -> TokenStream {
    gen_check_missing_fields(input)
}

fn gen_check_missing_fields(input: TokenStream) -> TokenStream {
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

fn get_use_pk(query: &Query) -> bool {
    match *query {
        Query::Delete { use_pk, .. } | Query::Select { use_pk, .. } | Query::Update { use_pk, .. } => use_pk,
        _ => false,
    }
}
