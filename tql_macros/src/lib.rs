//! The TQL library provide macros and attribute useful to generate SQL.
//!
//! The SQL is generated at compile time via a procedural macro.

#![feature(box_patterns, box_syntax, convert, plugin, plugin_registrar, quote, rustc_private)]
#![plugin(clippy)]
#![allow(ptr_arg)]

// TODO: use a mutex in each test to drop the tables even when the test fails.
// TODO: add support for Syntex.
// TODO: to benchmarks.

// TODO: span error when an SQL keyword is used in a table or field name (or renamed it?).
// TODO: do not use unwrap() in the generated code (unless this indicates a bug).
// TODO: add a warning for an update() without filters.
// TODO: support String methods in the update() method (for instance push(), push_str(), truncate(), pop(), remove()).
// TODO: improve ExprPath identifier extraction (check if there is only one segment).
// TODO: use all segments instead of only segments[0].
// TODO: find a way to stop the user from updating an item id.
// TODO: find a way to stop the user from accessing a related field when a join() is not done.
// TODO: add a step between the optimization and code generation to create a structure facilitating
// the code generation.
// FIXME: replace format!() by .to_owned() when possible.
// FIXME: remove useless clone().
// FIXME: use closures instead of internal functions.
// FIXME: use "use self" instead of two lines.
// TODO: implement the Default trait on table structures to be able to create a default object and
// only assign the fields that were fetched by the query (for only() and defer()).
// TODO: check byte strings (for instance: b"\u{a66e}").
// TODO: create different types for String (VARCHAR, CHAR(n), TEXT, …).
// TODO: make the error messages similar to Rust ones.
// TODO: make private most module identifiers.
// TODO: use unwrap() and unreachable!() to panics the compiler when there is a bug.
// TODO: support more database management systems.
// TODO: support methods on Nullable<Generic> and Nullable<i32> and other?
// TODO: support slices (for istance: Table.filter(field1[3..6] == "te")).
// TODO: add the method in() (for instance: Table.filter(field1.in([3, 4, 5]) ou Table.filter(field1.len().in(3..6)))).
// TODO: in aggregates, allow operations:
// Table.aggregate(avg(field2 / field1))
// TODO: check argument types in aggregations.
// TODO: in aggregates, allow selecting other fields (grouped fields only?).
// TODO: add the annotate() method for object aggregates.
// TODO: in aggregate filters, allow aggregate function calls.
// TODO: make more similar filters and aggregate filters to avoid code duplicate.
// TODO: create a macro to choose a DBMS. Give an optional parameter to this macro to choose the
// name of the macro to create (to allow using many DBMS at the same time).
// TODO: use a 2-pass compilation to detect used fields and joins (perhaps using a lint plugin).
// TODO: support compound primary keys.
// TODO: remove allow attributes that were added because of clippy bugs.

#[macro_use]
extern crate rustc;
extern crate syntax;

use rustc::lint::{EarlyLintPassObject, LateLintPassObject};
use rustc::plugin::Registry;
use syntax::ast::{AngleBracketedParameters, AngleBracketedParameterData, Block, Field, Ident, MetaItem, Path, PathSegment, StructField_, StructFieldKind, TokenTree, Ty, Ty_, VariantData, Visibility};
use syntax::ast::Expr_::ExprLit;
use syntax::ast::Item_::ItemStruct;
use syntax::ast::MetaItem_::MetaWord;
use syntax::codemap::{BytePos, Span, Spanned};
use syntax::ext::base::{Annotatable, DummyResult, ExtCtxt, MacEager, MacResult};
use syntax::ext::base::Annotatable::Item;
use syntax::ext::base::SyntaxExtension::MultiDecorator;
use syntax::ext::build::AstBuilder;
use syntax::ext::deriving::debug::expand_deriving_debug;
use syntax::owned_slice::OwnedSlice;
use syntax::parse::token::{InternedString, Token, intern, str_to_ident};
use syntax::ptr::P;

#[macro_use]
pub mod hashmap;
pub mod analyzer;
pub mod arguments;
pub mod ast;
pub mod attribute;
pub mod error;
pub mod gen;
pub mod methods;
pub mod optimizer;
pub mod parser;
pub mod plugin;
pub mod sql;
pub mod state;
pub mod string;
pub mod type_analyzer;
pub mod types;

pub type SqlQueryWithArgs = (String, QueryType, Args, Vec<Join>, Vec<Aggregate>);

use analyzer::{analyze, analyze_types, has_joins};
use arguments::{Args, arguments};
use ast::{Aggregate, Expression, Join, Query, QueryType, query_type};
use attribute::fields_vec_to_hashmap;
use error::{Error, ErrorType, SqlResult};
use gen::ToSql;
use optimizer::optimize;
use parser::parse;
use plugin::NODE_ID;
use state::{SqlArg, SqlArgs, SqlFields, SqlTable, SqlTables, get_primary_key_field_by_table_name, lint_singleton, singleton};
use type_analyzer::{SqlAttrError, SqlError};
use types::Type;

/// Add a `Field` made with the `expr`, identified by `name` at `position`.
fn add_field(fields: &mut Vec<Field>, expr: Expression, name: &str, position: Span) {
    fields.push(Field {
        expr: expr,
        ident: Spanned {
            node: str_to_ident(name),
            span: position,
        },
        span: position,
    });
}

/// Expand the `sql!()` macro.
/// This macro converts the Rust code provided as argument to SQL and outputs Rust code using the
/// `postgres` library.
fn expand_sql(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'static> {
    // TODO: if the first parameter is not provided, use "connection".
    let sql_result = to_sql(cx, args);
    match sql_result {
        Ok(sql_query_with_args) => {
            if let TokenTree::Token(_, Token::Ident(ident, _)) = args[0] {
                gen_query(cx, sp, ident, sql_query_with_args)
            }
            else {
                cx.span_err(sp, "Expected table identifier"); // TODO: improve this message.
                DummyResult::any(sp)
            }
        },
        Err(errors) => {
            span_errors(errors, cx);
            DummyResult::any(sp)
        },
    }
}

/// Expand the `#[SqlTable]` attribute.
/// This attribute must be used on structs to tell tql that it represents an SQL table.
// TODO: divide this function in many functions.
#[allow(cmp_owned)]
fn expand_sql_table(cx: &mut ExtCtxt, sp: Span, meta_item: &MetaItem, annotatable: &Annotatable, push: &mut FnMut(Annotatable)) {
    // Add to sql_tables.
    let mut sql_tables = singleton();

    // Add the #[derive(Debug)] attribute if needed.
    let attrs = annotatable.attrs();
    if let &Item(_) = annotatable {
        if attrs.iter().all(|item| {
                if let MetaWord(ref word) = item.node.value.node {
                    return word.to_string() != "derive_Debug"
                }
                true
            }) {
            expand_deriving_debug(cx, sp, meta_item, annotatable, push);
        }
    }

    if let &Annotatable::Item(ref item) = annotatable {
        if let ItemStruct(ref struct_def, _) = item.node {
            let table_name = item.ident.to_string();
            if !sql_tables.contains_key(&table_name) {
                let fields = fields_vec_to_hashmap(struct_def.fields());
                for field in fields.values() {
                    match field.node {
                        Type::UnsupportedType(ref typ) | Type::Nullable(box Type::UnsupportedType(ref typ)) =>
                            cx.parse_sess.span_diagnostic.span_err_with_code(field.span, &format!("use of unsupported type name `{}`", typ), "E0412"),
                        _ => (), // NOTE: Other types are supported.
                    }
                }

                sql_tables.insert(table_name.clone(), SqlTable {
                    fields: fields,
                    name: table_name.clone(),
                    position: item.span,
                });

                // Add the postgres::types::ToSql implementation for the struct.
                // Its SQL representation is the same as the primary key SQL representation.
                match get_primary_key_field_by_table_name(&table_name) {
                    Some(primary_key_field) => {
                        let table_ident = str_to_ident(&table_name);
                        let primary_key_ident = str_to_ident(&primary_key_field);
                        let implementation = quote_item!(cx,
                            impl postgres::types::ToSql for $table_ident {
                                fn to_sql<W: std::io::Write + ?Sized>(&self, ty: &postgres::types::Type, out: &mut W, ctx: &postgres::types::SessionInfo) -> postgres::Result<postgres::types::IsNull> {
                                    self.$primary_key_ident.to_sql(ty, out, ctx)
                                }

                                fn accepts(ty: &postgres::types::Type) -> bool {
                                    match *ty {
                                        postgres::types::Type::Int4 => true,
                                        _ => false,
                                    }
                                }

                                fn to_sql_checked(&self, ty: &postgres::types::Type, out: &mut ::std::io::Write, ctx: &postgres::types::SessionInfo) -> postgres::Result<postgres::types::IsNull> {
                                    if !<Self as postgres::types::ToSql>::accepts(ty) {
                                        return Err(postgres::error::Error::WrongType(ty.clone()));
                                    }
                                    self.to_sql(ty, out, ctx)
                                }
                            }
                        );
                        push(Annotatable::Item(implementation.unwrap()));
                    },
                    None => (), // NOTE: Do not add the implementation when there is no primary key.
                }
            }
            else {
                cx.parse_sess.span_diagnostic.span_err_with_code(item.span, &format!("duplicate definition of table `{}`", table_name), "E0428");
            }
        }
        else {
            cx.span_err(item.span, "Expected struct but found"); // TODO: improve this message.
        }
    }
    else {
        cx.span_err(sp, "Expected struct item"); // TODO: improve this message.
    }
}

/// Expand the `to_sql!()` macro.
/// This macro converts the Rust code provided as argument to SQL and ouputs it as a string
/// expression.
fn expand_to_sql(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'static> {
    let sql_result = to_sql(cx, args);
    match sql_result {
        Ok((sql, _, _, _, _)) => {
            let string_literal = intern(&sql);
            MacEager::expr(cx.expr_str(sp, InternedString::new_from_name(string_literal)))
        },
        Err(errors) => {
            span_errors(errors, cx);
            DummyResult::any(sp)
        },
    }
}

/// Generate the aggregate struct and struct expression.
fn gen_aggregate_struct(cx: &mut ExtCtxt, sp: Span, aggregates: &[Aggregate]) -> P<Block> {
    let mut aggregate_fields = vec![];
    let mut fields = vec![];
    for (index, aggregate) in aggregates.iter().enumerate() {
        let field_name = aggregate.result_name.clone();
        add_field(&mut aggregate_fields, quote_expr!(cx, row.get($index)), &field_name, sp);
        fields.push(Spanned {
            node: StructField_ {
                kind: StructFieldKind::NamedField(str_to_ident(&field_name), Visibility::Inherited),
                id: NODE_ID,
                ty: P(Ty {
                    id: NODE_ID,
                    node: Ty_::TyPath(None, Path {
                        span: sp,
                        global: false,
                        segments: vec![PathSegment {
                            identifier: str_to_ident("i32"), // TODO: choose the type from the field?
                            parameters: AngleBracketedParameters(AngleBracketedParameterData {
                                bindings: OwnedSlice::empty(),
                                lifetimes: vec![],
                                types: OwnedSlice::empty(),
                            }),
                        }],
                    }),
                    span: sp,
                }),
                attrs: vec![],
            },
            span: sp,
        });
    }
    let struct_ident = str_to_ident("Aggregate");
    let aggregate_struct = cx.item_struct(sp, struct_ident, VariantData::Struct(fields, NODE_ID));
    let aggregate_stmt = cx.stmt_item(sp, aggregate_struct);
    let instance = cx.expr_struct(sp, cx.path_ident(sp, struct_ident), aggregate_fields);
    cx.block(sp, vec![aggregate_stmt], Some(instance))
}

/// Generate the Rust code from the SQL query.
fn gen_query(cx: &mut ExtCtxt, sp: Span, table_ident: Ident, sql_query_with_args: SqlQueryWithArgs) -> Box<MacResult + 'static> {
    let (sql, query_type, arguments, joins, aggregates) = sql_query_with_args;
    let string_literal = intern(&sql);
    let sql_query = cx.expr_str(sp, InternedString::new_from_name(string_literal));
    let ident = Ident::new(intern("connection"), table_ident.ctxt);
    let sql_tables = singleton();
    let table_name = table_ident.to_string();
    match sql_tables.get(&table_name) {
        Some(table) => {
            let fields = get_query_fields(cx, sp, &table.fields, sql_tables, joins);
            let struct_expr = cx.expr_struct(sp, cx.path_ident(sp, table_ident), fields);
            let aggregate_struct = gen_aggregate_struct(cx, sp, &aggregates);
            let args_expr = get_query_arguments(cx, sp, table_name, arguments);
            let expr = gen_query_expr(cx, ident, sql_query, args_expr, struct_expr, aggregate_struct, query_type);
            MacEager::expr(expr)
        },
        None => DummyResult::any(sp),
    }
}

/// Generate the Rust code using the `postgres` library depending on the `QueryType`.
fn gen_query_expr(cx: &mut ExtCtxt, ident: Ident, sql_query: Expression, args_expr: Expression, struct_expr: Expression, aggregate_struct: P<Block>, query_type: QueryType) -> Expression {
    match query_type {
        QueryType::AggregateMulti => {
            quote_expr!(cx, {
                let result = $ident.prepare($sql_query).unwrap();
                // TODO: return an iterator instead of a vector.
                result.query(&$args_expr).unwrap().iter().map(|row| {
                    $aggregate_struct
                }).collect::<Vec<_>>()
            })
        },
        QueryType::AggregateOne => {
            quote_expr!(cx, {
                let result = $ident.prepare($sql_query).unwrap();
                result.query(&$args_expr).unwrap().iter().next().map(|row| {
                    $aggregate_struct
                })
            })
        },
        QueryType::InsertOne => {
            quote_expr!(cx, {
                $ident.prepare($sql_query)
                    .and_then(|result| {
                        // NOTE: The query is not supposed to fail, hence unwrap().
                        let rows = result.query(&$args_expr).unwrap();
                        // NOTE: There is always one result (the inserted id), hence unwrap().
                        let row = rows.iter().next().unwrap();
                        let count: i32 = row.get(0);
                        Ok(count)
                    })
            })
        },
        QueryType::SelectMulti => {
            quote_expr!(cx, {
                let result = $ident.prepare($sql_query).unwrap();
                // TODO: return an iterator instead of a vector.
                result.query(&$args_expr).unwrap().iter().map(|row| {
                    $struct_expr
                }).collect::<Vec<_>>()
            })
        },
        QueryType::SelectOne => {
            quote_expr!(cx, {
                let result = $ident.prepare($sql_query).unwrap();
                result.query(&$args_expr).unwrap().iter().next().map(|row| {
                    $struct_expr
                })
            })
        },
        QueryType::Exec => {
            quote_expr!(cx, {
                $ident.prepare($sql_query)
                    .and_then(|result| result.execute(&$args_expr))
            })
        },
    }
}

/// Get the arguments to send to the `postgres::stmt::Statement::query` or
/// `postgres::stmt::Statement::execute` method.
fn get_query_arguments(cx: &mut ExtCtxt, sp: Span, table_name: String, arguments: Args) -> Expression {
    let mut arg_refs = vec![];
    let mut sql_args = vec![];
    let calls = lint_singleton();

    for arg in arguments {
        let pos = arg.expression.span;

        let (low, high) =
            match (pos.lo, pos.hi) {
                (BytePos(low), BytePos(high)) => (low, high),
            };
        sql_args.push(SqlArg {
            high: high,
            low: low,
            typ: arg.typ,
        });

        match arg.expression.node {
            // Do not add literal arguments as they are in the final string literal.
            ExprLit(_) => (),
            _ => {
                arg_refs.push(cx.expr_addr_of(sp, arg.expression));
            },
        }
    }

    let BytePos(low) = sp.lo;
    calls.insert(low, SqlArgs {
        arguments: sql_args,
        table_name: table_name.to_owned(),
    });

    cx.expr_vec(sp, arg_refs)
}

/// Get the fully qualified field names for the struct expression needed by the generated code.
fn get_query_fields(cx: &mut ExtCtxt, sp: Span, table: &SqlFields, sql_tables: &SqlTables, joins: Vec<Join>) -> Vec<Field> {
    let mut fields = vec![];
    let mut index = 0usize;
    for (name, typ) in table {
        match typ.node {
            Type::Custom(ref foreign_table) => {
                let table_name = foreign_table;
                if let Some(foreign_table) = sql_tables.get(foreign_table) {
                    if has_joins(&joins, name) {
                        let mut foreign_fields = vec![];
                        for (field, typ) in &foreign_table.fields {
                            match typ.node {
                                Type::Custom(_) | Type::UnsupportedType(_) => (), // Do not add foreign key recursively.
                                _ => {
                                    add_field(&mut foreign_fields, quote_expr!(cx, row.get($index)), &field, sp);
                                    index += 1;
                                },
                            }
                        }
                        let related_struct = cx.expr_struct(sp, cx.path_ident(sp, str_to_ident(table_name)), foreign_fields);
                        add_field(&mut fields, quote_expr!(cx, Some($related_struct)), name, sp);
                    }
                    else {
                        // Since a `ForeignKey` is an `Option`, we output `None` when the field
                        // is not `join`ed.
                        add_field(&mut fields, quote_expr!(cx, None), name, sp);
                    }
                }
                // NOTE: if the field type is not an SQL table, an error is thrown by the linter.
            },
            Type::UnsupportedType(_) => (),
            _ => {
                add_field(&mut fields, quote_expr!(cx, row.get($index)), name, sp);
                index += 1;
            },
        }
    }
    fields
}

/// Show the compilation errors.
fn span_errors(errors: Vec<Error>, cx: &mut ExtCtxt) {
    for &Error {ref code, ref message, position, ref kind} in &errors {
        match *kind {
            ErrorType::Error => {
                match *code {
                    Some(ref code) => cx.parse_sess.span_diagnostic.span_err_with_code(position, &message, code),
                    None => cx.span_err(position, &message),
                }
            },
            ErrorType::Help => {
                cx.parse_sess.span_diagnostic.fileline_help(position, &message);
            },
            ErrorType::Note => {
                cx.parse_sess.span_diagnostic.fileline_note(position, &message);
            },
            ErrorType::Warning => {
                cx.span_warn(position, &message);
            },
        }
    }
}

/// Convert the Rust code to an SQL string with its type, arguments and joins.
fn to_sql(cx: &mut ExtCtxt, args: &[TokenTree]) -> SqlResult<SqlQueryWithArgs> {
    if args.is_empty() {
        return Err(vec![Error::new_with_code("this macro takes 1 parameter but 0 parameters were supplied".to_owned(), cx.call_site(), "E0061")]);
    }

    let mut parser = cx.new_parser_from_tts(args);
    let expression = parser.parse_expr_panic();
    let sql_tables = singleton();
    let method_calls = try!(parse(expression));
    let mut query = try!(analyze(method_calls, sql_tables));
    optimize(&mut query);
    query = try!(analyze_types(query));
    let sql = query.to_sql();
    let joins =
        match query {
            Query::Select { ref joins, .. } => joins.clone(),
            _ => vec![],
        };
    let aggrs: Vec<Aggregate> =
        match query {
            Query::Aggregate { ref aggregates, .. } => aggregates.clone(),
            _ => vec![],
        };
    Ok((sql, query_type(&query), arguments(cx, query), joins, aggrs))
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("to_sql", expand_to_sql);
    reg.register_macro("sql", expand_sql);
    reg.register_syntax_extension(intern("SqlTable"), MultiDecorator(box expand_sql_table));
    reg.register_early_lint_pass(box SqlAttrError as EarlyLintPassObject);
    reg.register_late_lint_pass(box SqlError as LateLintPassObject);
}
