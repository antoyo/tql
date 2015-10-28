//! The TQL library provide macros and attribute useful to generate SQL.
//!
//! The SQL is generated at compile time via a procedural macro.

#![feature(box_patterns, box_syntax, convert, plugin, plugin_registrar, quote, rustc_private)]
#![plugin(clippy)]
#![warn(option_unwrap_used, result_unwrap_used)]

// TODO: changer le courriel de l’auteur avant de mettre sur Github.

// TODO: supporter les méthodes sur Nullable<Generic> et Nullable<i32> et autres?
// TODO: erreur pour les types Option<Option<_>>.
// TODO: ne pas faire d’erreur pour un type Option<Unsupported> quand il est oublié dans insert().
// TODO: avertissement pour un delete() sans filtre.
// TODO: retourner l’élément inséré par l’appel à la méthode insert().
// TODO: paramétriser le type ForeignKey et PrimaryKey pour que la macro puisse choisir de mettre
// le type en question ou rien (dans le cas où la jointure n’est pas faite) ou empêcher les
// modifications (dans le cas où l’ID existe).
// TODO: ajouter une étape entre l’optimisation et la génération de code pour produire une
// structure qui facilitera la génération du code.
// TODO: utiliser tous les segments au lieu de juste segments[0].
// FIXME: unreachable!() fait planter le compilateur.
// FIXME: remplacer format!() par .to_owned() quand c’est possible.
// FIXME: enlever les clone() inutiles.
// FIXME: utiliser des fermetures à la place de fonctions internes.
// FIXME: utiliser use self au lieu de deux lignes.
// TODO: créer différents types pour String (VARCHAR, CHAR(n), TEXT, …).
// TODO: rendre les messages d’erreur plus semblables à ceux de Rust.
// TODO: rendre le moins d’identifiants publiques.
// TODO: supporter plusieurs SGBDs.
// TODO: faire des benchmarks.
// TODO: créer une macro qui permet de choisir le SGBD. Donner un paramètre optionel à cette macro
// pour choisir le nom de la macro à créer (pour permettre d’utiliser plusieurs SGBDs à la fois).
// TODO: utiliser une compilation en 2 passes pour détecter les champs utilisés et les jointures
// utiles (peut-être possible avec un lint plugin).
// TODO: peut-être utiliser Spanned pour conserver la position dans l’AST.
// TODO: permetre les opérateurs += et autre pour un update.
// TODO: supporter les clés primaires composées.
// TODO: supporter la comparaison avec une clé étrangère :
// impl postgres::types::ToSql for ForeignTable {
//    fn to_sql<W: std::io::Write + ?Sized>(&self, ty: &postgres::types::Type, out: &mut W, ctx: &postgres::types::SessionInfo) -> postgres::Result<postgres::types::IsNull> {
//        try!(out.write(self.id.to_string().as_bytes()));
//        Ok(postgres::types::IsNull::No)
//    }
//
//    accepts!(postgres::types::Type::Oid);
//
//    to_sql_checked!();
//}

#[macro_use]
extern crate rustc;
extern crate syntax;

use rustc::lint::{EarlyLintPassObject, LateLintPassObject};
use rustc::plugin::Registry;
use syntax::ast::{Field, Ident, MetaItem, TokenTree};
use syntax::ast::Expr_::ExprLit;
use syntax::ast::Item_::ItemStruct;
use syntax::codemap::{DUMMY_SP, BytePos, Span, Spanned};
use syntax::ext::base::{Annotatable, DummyResult, ExtCtxt, MacEager, MacResult};
use syntax::ext::base::SyntaxExtension::MultiDecorator;
use syntax::ext::build::AstBuilder;
use syntax::parse::token::{InternedString, Token, intern, str_to_ident};

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

pub type SqlQueryWithArgs = (String, QueryType, Args, Vec<Join>);

use analyzer::{analyze, analyze_types, has_joins};
use arguments::{Args, arguments};
use ast::{Expression, Join, Query, QueryType, query_type};
use attribute::fields_vec_to_hashmap;
use error::{Error, ErrorType, SqlResult};
use gen::ToSql;
use optimizer::optimize;
use parser::parse;
use state::{SqlArg, SqlArgs, SqlFields, SqlTables, lint_singleton, singleton};
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
    // TODO: si le premier paramètre n’est pas fourni, utiliser "connection".
    if let TokenTree::TtToken(_, Token::Ident(ident, _)) = args[0] {
        let sql_result = to_sql(cx, args);
        match sql_result {
            Ok(sql_query_with_args) => {
                gen_query(cx, sp, ident, sql_query_with_args)
            }
            Err(errors) => {
                span_errors(errors, cx);
                DummyResult::any(sp)
            }
        }
    }
    else {
        cx.span_err(sp, "Expected table identifier");
        DummyResult::any(sp)
    }
}

/// Expand the `#[sql_table]` attribute.
/// This attribute must be used on structs to tell tql that it represents an SQL table.
fn expand_sql_table(cx: &mut ExtCtxt, sp: Span, _: &MetaItem, item: &Annotatable, _: &mut FnMut(Annotatable)) {
    // Add to sql_tables.
    let mut sql_tables = singleton();

    if let &Annotatable::Item(ref item) = item {
        if let ItemStruct(ref struct_def, _) = item.node {
            // TODO: vérifier le type des champs de la structure.
            // Pour ForeignKey, vérifier que le type T est une table existante.
            let table_name = item.ident.to_string();
            let fields = fields_vec_to_hashmap(struct_def.fields());
            for field in fields.values() {
                if let Type::UnsupportedType(ref typ) = field.node {
                    //panic!(format!("{:?}", field.node));
                    cx.parse_sess.span_diagnostic.span_err_with_code(field.span, &format!("use of unsupported type name `{}`", typ), "E0412");
                }
            }
            sql_tables.insert(table_name, fields);
        }
        else {
            // TODO
            cx.span_err(item.span, "Expected struct but found");
        }
    }
    else {
        cx.span_err(sp, "Expected struct item");
    }
}

/// Expand the `to_sql!()` macro.
/// This macro converts the Rust code provided as argument to SQL and ouputs it as a string
/// expression.
fn expand_to_sql(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'static> {
    let sql_result = to_sql(cx, args);
    match sql_result {
        Ok((sql, _, _, _)) => {
            let string_literal = intern(&sql);
            MacEager::expr(cx.expr_str(sp, InternedString::new_from_name(string_literal)))
        }
        Err(errors) => {
            span_errors(errors, cx);
            DummyResult::any(sp)
        }
    }
}

/// Generate the Rust code from the SQL query.
fn gen_query(cx: &mut ExtCtxt, sp: Span, table_ident: Ident, sql_query_with_args: SqlQueryWithArgs) -> Box<MacResult + 'static> {
    let (sql, query_type, arguments, joins) = sql_query_with_args;
    let string_literal = intern(&sql);
    let sql_query = cx.expr_str(sp, InternedString::new_from_name(string_literal));
    let ident = Ident::new(intern("connection"), table_ident.ctxt);
    let sql_tables = singleton();
    let table_name = table_ident.to_string();
    match sql_tables.get(&table_name) {
        Some(table) => {
            let fields = get_query_fields(cx, sp, table, sql_tables, joins);
            let struct_expr = cx.expr_struct(sp, cx.path_ident(sp, table_ident), fields);
            let args_expr = get_query_arguments(cx, sp, table_name, arguments);
            let expr = gen_query_expr(cx, ident, sql_query, args_expr, struct_expr, query_type);
            MacEager::expr(expr)
        },
        None => DummyResult::any(sp),
    }
}

/// Generate the Rust code using the `postgres` library depending on the `QueryType`.
fn gen_query_expr(cx: &mut ExtCtxt, ident: Ident, sql_query: Expression, args_expr: Expression, struct_expr: Expression, query_type: QueryType) -> Expression {
    match query_type {
        QueryType::SelectMulti => {
            quote_expr!(cx, {
                let result = $ident.prepare($sql_query).unwrap();
                // TODO: retourner un itérateur au lieu d’un vecteur.
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
                let result = $ident.prepare($sql_query).unwrap();
                result.execute(&$args_expr)
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
                arg_refs.push(cx.expr_addr_of(DUMMY_SP, arg.expression));
            },
        }
    }

    let BytePos(low) = sp.lo;
    calls.insert(low, SqlArgs {
        arguments: sql_args,
        table_name: table_name.to_owned(),
    });

    cx.expr_vec(DUMMY_SP, arg_refs)
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
                        for (field, typ) in foreign_table {
                            match typ.node {
                                Type::Custom(_) | Type::UnsupportedType(_) => (), // Do not add foreign key recursively.
                                _ => {
                                    add_field(&mut foreign_fields, quote_expr!(cx, row.get($index)), field, sp);
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
    let expression = parser.parse_expr();
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
    Ok((sql, query_type(&query), arguments(cx, query), joins))
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("to_sql", expand_to_sql);
    reg.register_macro("sql", expand_sql);
    reg.register_syntax_extension(intern("SqlTable"), MultiDecorator(box expand_sql_table));
    reg.register_early_lint_pass(box SqlAttrError as EarlyLintPassObject);
    reg.register_late_lint_pass(box SqlError as LateLintPassObject);
}
