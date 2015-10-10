//! The TQL library provide macros and attribute useful to generate SQL.
//!
//! The SQL is generated at compile time via a procedural macro.

#![feature(plugin_registrar, quote, rustc_private)]

// FIXME: unreachable!() fait planter le compilateur.
// FIXME: remplacer format!() par .to_string() quand c’est possible.
// FIXME: enlever les clone() inutiles.
// FIXME: utiliser des fermetures à la place de fonctions internes.
// TODO: rendre les messages d’erreur plus semblables à ceux de Rust.
// TODO: rendre le moins d’identifiants publiques.
// TODO: supporter plusieurs SGBDs.
// TODO: faire des benchmarks.
// TODO: créer une macro qui permet de choisir le SGBD. Donner un paramètre optionel à cette macro
// pour choisir le nom de la macro à créer (pour permettre d’utiliser plusieurs SGBDs à la fois).
// TODO: utiliser une compilation en 2 passes pour détecter les champs utilisés et les jointures
// utiles (peut-être possible avec un lint plugin).
// TODO: utiliser un lint plugin pour afficher les erreurs sémantiques (enregistrer les structures
// avec leur positions dans un fichier qui sera utilisé dans le lint plugin pour afficher les
// erreurs aux bons endroits).
// TODO: peut-être utiliser Spanned pour conserver la position dans l’AST.

extern crate rustc;
extern crate syntax;

use rustc::plugin::Registry;
use syntax::ast::{Expr, Field, Ident, MetaItem, TokenTree};
use syntax::ast::Expr_::ExprLit;
use syntax::ast::Item_::ItemStruct;
use syntax::codemap::{DUMMY_SP, Span, Spanned};
use syntax::ext::base::{Annotatable, DummyResult, ExtCtxt, MacEager, MacResult};
use syntax::ext::base::SyntaxExtension::MultiDecorator;
use syntax::ext::build::AstBuilder;
use syntax::parse::token::{InternedString, Token, intern, str_to_ident};
use syntax::ptr::P;

pub mod analyzer;
pub mod ast;
pub mod attribute;
pub mod error;
pub mod gen;
pub mod optimizer;
pub mod parser;
pub mod plugin;
pub mod sql;
pub mod state;

pub type SqlQueryWithArgs = (String, QueryType, Vec<P<Expr>>);

use analyzer::analyze;
use ast::{Expression, FilterExpression, Limit, Query, QueryType, query_type};
use attribute::fields_vec_to_hashmap;
use error::{Error, SqlResult};
use gen::ToSql;
use optimizer::optimize;
use parser::parse;
use plugin::to_expr;
use state::singleton;

/// Extract the Rust `Expression`s from the `Query`.
fn arguments(cx: &mut ExtCtxt, query: Query) -> Vec<P<Expr>> {
    let mut arguments = vec![];

    fn is_literal(expr: &P<Expr>) -> bool {
        match expr.node {
            ExprLit(_) => true,
            _ => false,
        }
    }

    fn add_expr(arguments: &mut Vec<P<Expr>>, expr: P<Expr>) {
        if !is_literal(&expr) {
            arguments.push(expr);
        }
    }

    fn add(arguments: &mut Vec<P<Expr>>, expr: Expression) {
        add_expr(arguments, to_expr(expr))
    }

    fn add_filter_arguments(filter: FilterExpression, arguments: &mut Vec<P<Expr>>) {
        match filter {
            FilterExpression::Filter(filter) => {
                add(arguments, filter.operand2);
            },
            FilterExpression::Filters(filters) => {
                add_filter_arguments(*filters.operand1, arguments);
                add_filter_arguments(*filters.operand2, arguments);
            },
            FilterExpression::NoFilters => (),
        }
    }

    fn add_limit_arguments(cx: &mut ExtCtxt, limit: Limit, arguments: &mut Vec<P<Expr>>) {
        match limit {
            Limit::EndRange(expression) => add(arguments, expression),
            Limit::Index(expression) => add(arguments, expression),
            Limit::LimitOffset(_, _) => (),
            Limit::NoLimit => (),
            Limit::Range(expression1, expression2) => {
                let offset = expression1.clone();
                add(arguments, expression1);
                let expr2 = to_expr(expression2);
                let offset = to_expr(offset);
                add_expr(arguments, quote_expr!(cx, $expr2 - $offset));
            },
            Limit::StartRange(expression) => add(arguments, expression),
        }
    }

    match query {
        Query::Select {filter, limit, ..} => {
            add_filter_arguments(filter, &mut arguments);
            add_limit_arguments(cx, limit, &mut arguments);
        },
        _ => (),
    }

    arguments
}

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

fn expand_sql_table(cx: &mut ExtCtxt, sp: Span, _: &MetaItem, item: &Annotatable, _: &mut FnMut(Annotatable)) {
    // Add to sql_tables.
    let mut sql_tables = singleton();

    if let &Annotatable::Item(ref item) = item {
        if let ItemStruct(ref struct_def, _) = item.node {
            let table_name = item.ident.to_string();
            let fields = fields_vec_to_hashmap(&struct_def.fields);
            sql_tables.insert(table_name, fields);
        }
        else {
            cx.span_err(item.span, "Expected struct but found");
        }
    }
    else {
        cx.span_err(sp, "Expected struct item");
    }
}

fn expand_to_sql(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'static> {
    let sql_result = to_sql(cx, args);
    match sql_result {
        Ok((sql, _, _)) => {
            let string_literal = intern(&sql);
            MacEager::expr(cx.expr_str(sp, InternedString::new_from_name(string_literal)))
        }
        Err(errors) => {
            span_errors(errors, cx);
            DummyResult::any(sp)
        }
    }
}

fn gen_query(cx: &mut ExtCtxt, sp: Span, table_ident: Ident, sql_query_with_args: SqlQueryWithArgs) -> Box<MacResult + 'static> {
    // TODO: générer un code différent en fonction du query_type.
    let (sql, query_type, arguments) = sql_query_with_args;
    let string_literal = intern(&sql);
    let string = cx.expr_str(sp, InternedString::new_from_name(string_literal));
    let ident = Ident::new(intern("connection"), table_ident.ctxt);
    // TODO: utiliser un itérateur.
    let sql_tables = singleton();
    let table = sql_tables.get(&table_ident.to_string()).unwrap();
    let mut fields = vec![];
    // TODO: prendre en compte l’ID.
    let mut index = 0usize;

    for (name, _) in table {
        fields.push(Field {
            expr: quote_expr!(cx, row.get($index)),
            ident: Spanned {
                node: str_to_ident(name),
                span: sp,
            },
            span: sp,
        });
        index += 1;
    }

    let struct_expr = cx.expr_struct(sp, cx.path_ident(sp, table_ident), fields);

    let mut arg_refs = vec![];

    for arg in arguments {
        match arg.node {
            // Do not add literal arguments as they are in the final string literal.
            ExprLit(_) => (),
            _ => arg_refs.push(cx.expr_addr_of(DUMMY_SP, arg)),
        }
    }
    let args_expr = cx.expr_vec(DUMMY_SP, arg_refs);

    let expr = match query_type {
        QueryType::SelectMulti => {
            quote_expr!(cx, {
                let result = $ident.prepare($string).unwrap();
                result.query(&$args_expr).unwrap().iter().map(|row| {
                    $struct_expr
                }).collect::<Vec<_>>()
            })
        },
        QueryType::SelectOne => {
            quote_expr!(cx, {
                let result = $ident.prepare($string).unwrap();
                result.query(&$args_expr).unwrap().iter().next().map(|row| {
                    $struct_expr
                })
            })
        },
        QueryType::Exec => {
            quote_expr!(cx, {
                let result = $ident.prepare($string).unwrap();
                result.execute(&$args_expr)
            })
        },
    };

    MacEager::expr(expr)
}

fn span_errors(errors: Vec<Error>, cx: &mut ExtCtxt) {
    for &Error {ref message, position} in &errors {
        cx.span_err(position, &message);
    }
}

fn to_sql(cx: &mut ExtCtxt, args: &[TokenTree]) -> SqlResult<SqlQueryWithArgs> {
    let mut parser = cx.new_parser_from_tts(args);
    let expression = (*parser.parse_expr()).clone();
    let sql_tables = singleton();
    let method_calls = try!(parse(&expression));
    let mut query = try!(analyze(method_calls, sql_tables));
    query = optimize(query);
    let sql = query.to_sql();
    Ok((sql, query_type(&query), arguments(cx, query)))
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("to_sql", expand_to_sql);
    reg.register_macro("sql", expand_sql);
    reg.register_syntax_extension(intern("sql_table"), MultiDecorator(Box::new(expand_sql_table)));
}
