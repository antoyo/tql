/*
 * TODO: add travis (with special case for stable).
 * TODO: improve the error handling of the generated code.
 * TODO: use as_ref() for Ident instead of &ident.to_string().
 * TODO: the error message sometimes show String instead of &str.
 * FIXME: warning should not be errors on stable.
 */

#![cfg_attr(feature = "unstable", feature(proc_macro))]
#![recursion_limit="128"]

extern crate literalext;
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
#[cfg(feature = "unstable")]
use std::mem;

use proc_macro::TokenStream;
#[cfg(feature = "unstable")]
use proc_macro::{TokenNode, TokenTree};
use proc_macro2::{Literal, Term};
use quote::Tokens;
#[cfg(feature = "unstable")]
use quote::ToTokens;
use rand::Rng;
use syn::{
    AngleBracketedGenericArguments,
    ExprKind,
    ExprTup,
    ExprTupField,
    GenericArgument,
    Ident,
    Item,
    ItemEnum,
    ItemStruct,
    Lit,
    LitKind,
    Macro,
    Span,
    TypePath,
    VariantData,
    parse,
};
use syn::PathArguments::AngleBracketed;

use analyzer::{analyze, analyze_types, has_joins};
use arguments::{Args, arguments};
use ast::{
    Aggregate,
    Join,
    Query,
    QueryType,
    item_span,
    query_type,
};
#[cfg(feature = "unstable")]
use ast::{generic_arg_span, expr_span};
use attribute::{field_ty_to_type, fields_vec_to_hashmap};
use error::{Error, Result, res};
#[cfg(not(feature = "unstable"))]
use error::compiler_error;
use gen::ToSql;
use optimizer::optimize;
use parser::Parser;
use state::{
    SqlFields,
    SqlTable,
    SqlTables,
    get_primary_key_field_by_table_name,
    tables_singleton,
};
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
    let sql_result = to_sql_query(input);
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
    match to_sql_query(input) {
        Ok(args) => {
            let expr = Lit {
                value: LitKind::Other(Literal::string(&args.sql)),
                span: args.span,
            };
            let gen = quote! {
                #expr
            };
            gen.into()
        },
        Err(errors) => generate_errors(errors),
    }
}

/// Convert the Rust code to an SQL string with its type, arguments, joins, and aggregate fields.
fn to_sql_query(input: TokenStream) -> Result<SqlQueryWithArgs> {
    // TODO: use this when it becomes stable.
    /*if input.is_empty() {
        return Err(vec![Error::new_with_code("this macro takes 1 parameter but 0 parameters were supplied", cx.call_site(), "E0061")]);
    }*/
    let expr =
        match parse(input) {
            Ok(expr) => expr,
            Err(error) => return Err(vec![Error::new(&error.to_string(), Span::default())]),
        };
    #[cfg(feature = "unstable")]
    let span = expr_span(&expr);
    let sql_tables = tables_singleton();
    let parser = Parser::new();
    let method_calls = parser.parse(&expr)?;
    let table_name = method_calls.name.clone().expect("table name in method_calls");
    let mut query = analyze(method_calls, sql_tables)?;
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
    // Add to sql_tables.
    let sql_tables = tables_singleton();

    let item: Item = parse(input).expect("parse expression in sql_table()");

    let gen =
        if let Item::Struct(item_struct) = item {
            let table_name = item_struct.ident.to_string();
            let (fields, impls) = get_struct_fields(&item_struct);
            let mut compiler_errors = quote! {};
            if !sql_tables.contains_key(&table_name) {
                if let Ok(fields) = fields {
                    sql_tables.insert(table_name.clone(), SqlTable {
                        fields,
                        name: item_struct.ident.clone(),
                        position: item_struct.ident.span, // TODO: check if it is the right position.
                    });

                    // NOTE: Transform the span by dummy spans to workaround this issue:
                    // https://github.com/rust-lang/rust/issues/42337
                    let code = tosql_impl(&item_struct).into();
                    #[cfg(feature = "unstable")]
                    let code = respan(code);
                    return concat_token_stream(code, impls);
                }
            }
            else {
                // NOTE: This error is needed because the code could have two table structs in
                // different modules.
                let error = Error::new(&format!("duplicate definition of table `{}`", table_name),
                    item_struct.ident.span);
                #[cfg(feature = "unstable")]
                error.emit_diagnostic();
                #[cfg(not(feature = "unstable"))]
                {
                    let error = compiler_error(&error);
                    compiler_errors = quote! {
                        #compiler_errors
                        #error
                    };
                }
            }
            if let Err(errors) = fields {
                // NOTE: insert dummy table to be able to show more errors.
                sql_tables.insert(table_name.clone(), SqlTable {
                    fields: BTreeMap::new(),
                    name: item_struct.ident.clone(),
                    position: item_struct.ident.span,
                });

                for error in errors {
                    add_error(error, &mut compiler_errors);
                }
            }
            concat_token_stream(compiler_errors.into(), impls)
        }
        else {
            let mut compiler_errors = quote! {};
            let error = Error::new("Expected struct but found", item_span(&item)); // TODO: improve this message.
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
fn get_struct_fields(item_struct: &ItemStruct) -> (Result<SqlFields>, TokenStream) {
    fn error(span: Span, typ: &str) -> Error {
        Error::new_with_code(&format!("use of unsupported type name `{}`", typ),
            span, "E0412")
    }

    let position = item_struct.ident.span;
    let mut impls: TokenStream = quote! {}.into();
    let mut errors = vec![];

    let fields =
        match item_struct.data {
            VariantData::Struct(ref fields, _) => fields.clone().into_vec(),
            _ => return (Err(vec![Error::new("Expected normal struct, found", position)]), empty_token_stream()), // TODO: improve this message.
        };
    let mut primary_key_count = 0;
    for field in &fields {
        if field.ident.is_some() {
            #[cfg(feature = "unstable")]
            let field_type = &field.ty;
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
                Type::Serial => primary_key_count += 1,
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
                        let field_pos = generic_arg_span(&field_type);
                        let span = to_proc_macro_span(field_pos);
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
    (res(fields, errors), impls)
}

/// Add the postgres::types::ToSql implementation on the struct.
/// Its SQL representation is the same as the primary key SQL representation.
fn tosql_impl(item_struct: &ItemStruct) -> Tokens {
    let table_ident = &item_struct.ident;
    let debug_impl = create_debug_impl(item_struct);
    match get_primary_key_field_by_table_name(&table_ident.to_string()) {
        Some(primary_key_field) => {
            let primary_key_ident = Ident::from(primary_key_field.as_ref());
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
        },
        None => quote! {}, // NOTE: Do not add the implementation when there is no primary key.
    }
}

fn create_debug_impl(item_struct: &ItemStruct) -> Tokens {
    let table_ident = &item_struct.ident;
    let table_name = table_ident.to_string();
    if let VariantData::Struct(ref fields, _) = item_struct.data {
        let fields = fields.clone();
        let field_idents = fields.iter()
            .map(|element| element.into_item())
            .map(|field| field.ident.expect("field has name"));
        let field_names = field_idents
            .map(|ident| ident.to_string());
        let field_idents = fields.iter()
            .map(|element| element.into_item())
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
        unimplemented!();
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
        let expr = Lit {
            value: LitKind::Other(Literal::string("")),
            span: Span::default(),
        };
        let gen = quote! {
            #expr
        };
        gen.into()
    }
}

/// Generate the Rust code from the SQL query.
fn gen_query(args: SqlQueryWithArgs) -> TokenStream {
    let table_ident = &args.table_name;
    let ident = Ident::new(Term::intern("connection"), table_ident.span);
    let sql_tables = tables_singleton();
    let table_name = table_ident.to_string();
    let tokens =
        match sql_tables.get(&table_name) {
            Some(table) => {
                let struct_expr = create_struct(table_ident, &table.fields, sql_tables, args.joins);
                let (aggregate_struct, aggregate_expr) = gen_aggregate_struct(&args.aggregates);
                let args_expr = get_query_arguments(args.arguments);
                gen_query_expr(ident, args.sql, args_expr, struct_expr, aggregate_struct, aggregate_expr, args.query_type)
            },
            None => quote! {},
        };
    tokens.into()
}

/// Generate the Rust code using the `postgres` library depending on the `QueryType`.
fn gen_query_expr(ident: Ident, sql_query: String, args_expr: Tokens, struct_expr: Tokens,
                  aggregate_struct: Tokens, aggregate_expr: Tokens, query_type: QueryType) -> Tokens
{
    let sql_query = ExprKind::Lit(Lit {
        value: LitKind::Other(Literal::string(&sql_query)),
        span: Span::default(),
    });
    match query_type {
        QueryType::AggregateMulti => {
            let result = quote! {{
                let result = #ident.prepare(#sql_query).unwrap();
                result.query(&#args_expr).unwrap().iter()
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
                let result = #ident.prepare(#sql_query).unwrap();
                result.query(&#args_expr).unwrap().iter().next().map(|row| {
                    #aggregate_expr
                })
            }}
        },
        QueryType::InsertOne => {
            quote! {{
                #ident.prepare(#sql_query)
                    .and_then(|result| {
                        // NOTE: The query is not supposed to fail, hence unwrap().
                        let rows = result.query(&#args_expr).unwrap();
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
                    let result = #ident.prepare(#sql_query).unwrap();
                    result.query(&#args_expr).unwrap().iter()
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
                    let result = #ident.prepare(#sql_query).unwrap();
                    result.query(&#args_expr).unwrap().iter().next()
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
                #ident.prepare(#sql_query)
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
    let mut types = vec![];
    let mut exprs = vec![];

    for arg in arguments {
        match arg.expression.node {
            // Do not add literal arguments as they are in the final string literal.
            ExprKind::Lit(_) => (),
            _ => {
                let expr = &arg.expression;
                arg_refs.push(quote! { &(#expr) });
            },
        }

        let name = arg.field_name.unwrap_or_else(|| rand_string().to_lowercase());
        idents.push(new_ident(&format!("__tql_{}", name)));

        let exp = &arg.expression;
        exprs.push(quote! { #exp });

        if is_string_type(&arg.typ) {
            qualifiers.push(quote! {});
        }
        else {
            qualifiers.push(quote! { ref });
        }

        if let Some(ty) = inner_type(&arg.typ) {
            types.push(quote! { #ty });
        }
        else if is_string_type(&arg.typ) {
            types.push(quote! { &str });
        }
        else {
            let ty = &arg.typ;
            types.push(quote! { #ty });
        }
    }

    quote! {{
        // Add the arguments as let statements so that they can be type-checked.
        // TODO: check that this let is not in the generated binary.
        #(let #qualifiers #idents: #types = #exprs;)*
        [#(#arg_refs),*]
    }}
}

/// Create the struct expression needed by the generated code.
fn create_struct(table_ident: &Ident, table: &SqlFields, sql_tables: &SqlTables, joins: Vec<Join>) -> Tokens {
    let mut field_idents = vec![];
    let mut field_values = vec![];
    let mut index = 0usize;
    for (name, types) in table {
        match types.ty.node {
            Type::Custom(ref foreign_table) => {
                if let Some(foreign_table) = sql_tables.get(foreign_table) {
                    if has_joins(&joins, name) {
                        // If there is a join, fetch the joined fields.
                        let mut foreign_field_idents = vec![];
                        let mut foreign_field_values = vec![];
                        for (field, types) in &foreign_table.fields {
                            match types.ty.node {
                                Type::Custom(_) | Type::UnsupportedType(_) => (), // Do not add foreign key recursively.
                                _ => {
                                    foreign_field_idents.push(field);
                                    foreign_field_values.push(quote! { row.get(#index) });
                                    index += 1;
                                },
                            }
                        }
                        let foreign_table_ident = &foreign_table.name;
                        let related_struct =
                            quote! {
                                #foreign_table_ident {
                                    #(#foreign_field_idents: #foreign_field_values),*
                                }
                            };
                        field_idents.push(name.clone());
                        field_values.push(quote! { Some(#related_struct) });
                    }
                    else {
                        // Since a `ForeignKey` is an `Option`, we output `None` when the field
                        // is not `join`ed.
                        field_idents.push(name.clone());
                        field_values.push(quote! { None });
                    }
                }
                // NOTE: if the field type is not an SQL table, an error is thrown by the linter.
            },
            Type::UnsupportedType(_) => (), // TODO: should panic.
            _ => {
                field_idents.push(name.clone());
                field_values.push(quote! { row.get(#index) });
                index += 1;
            },
        }
    }
    let code = quote! {
        #table_ident {
            #(#field_idents: #field_values),*
        }
    };
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
    Ident::new(Term::intern(string), Span::default())
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

#[cfg(feature = "unstable")]
fn to_proc_macro_span(span: Span) -> proc_macro::Span {
    // TODO: avoid using transmute.
    unsafe { mem::transmute(span) }
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

// Stable implementation.

// TODO: make this function more robust.
#[proc_macro_derive(StableToSql)]
pub fn stable_to_sql(input: TokenStream) -> TokenStream {
    let enumeration: Item = parse(input).unwrap();
    if let Item::Enum(ItemEnum { ref variants, .. }) = enumeration {
        let variant = &variants.first().unwrap().item().discriminant;
        if let ExprKind::TupField(ExprTupField { ref expr, .. }) = variant.as_ref().unwrap().node {
            if let ExprKind::Tup(ExprTup { ref args, .. }) = expr.node {
                if let ExprKind::Macro(Macro { ref tokens, .. }) = args.first().unwrap().item().node {
                    let tokens: proc_macro2::TokenStream = tokens[0].clone().0.into();
                    let tokens = tokens.to_string();
                    let tokens = tokens.trim();
                    let tokens = &tokens[1..tokens.len() - 1]; // Remove the parenthesis.
                    let tokens: TokenStream = std::str::FromStr::from_str(&tokens).unwrap();

                    let sql_result = to_sql_query(tokens);
                    let code = match sql_result {
                        Ok(sql_query_with_args) => gen_query(sql_query_with_args),
                        Err(errors) => generate_errors(errors),
                    };
                    let code = token_stream_to_tokens(code);

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

fn token_stream_to_tokens(tokens: TokenStream) -> Tokens {
    let mut result = quote! {
    };
    let tokens: proc_macro2::TokenStream = tokens.into();
    for token in tokens {
        result = quote! {
            #result
            #token
        };
    }
    result
}
