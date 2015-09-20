#![feature(plugin_registrar, rustc_private, slice_patterns)]

// TODO: permettre de spécifier les champs manuellement pour un SELECT.
// TODO: supporter plusieurs SGBDs.
// TODO: faire des benchmarks.
// TODO: créer une macro qui permet de choisir le SGBD. Donner un paramètre optionel à cette macro
// pour choisir le nom de la macro à créer (pour permettre d’utiliser plusieurs SGBDs à la fois).
// TODO: utiliser une compilation en 2 passes pour détecter les champs utilisés et les jointures
// utiles (peut-être possible avec un lint plugin).

extern crate rustc;
extern crate syntax;

use rustc::plugin::Registry;
use syntax::ast::{Expr_, MetaItem, TokenTree};
use syntax::ast::Expr_::{ExprMethodCall, ExprPath};
use syntax::codemap::{Span, Spanned};
use syntax::ext::base::{Annotatable, DummyResult, ExtCtxt, MacEager, MacResult};
use syntax::ext::base::SyntaxExtension::MultiDecorator;
use syntax::ext::build::AstBuilder;
use syntax::parse::token::{InternedString, intern};

use std::collections::HashSet;
use std::mem;

pub mod ast;
pub mod gen;

use ast::{Filter, Query};
use ast::convert::expression_to_filter;
use gen::ToSql;

type SqlTables = HashSet<String>;

// FIXME: make this thread safe.
fn singleton() -> &'static mut SqlTables {
    static mut hash_map: *mut SqlTables = 0 as *mut SqlTables;

    let map: SqlTables = HashSet::new();
    unsafe {
        if hash_map == 0 as *mut SqlTables {
            hash_map = mem::transmute(Box::new(map));
        }
        &mut *hash_map
    }
}

fn expand_select(cx: &mut ExtCtxt, expr: Expr_, filter: Option<Filter>) -> String {
    if let ExprPath(None, path) = expr {
        let table_name = path.segments[0].identifier.to_string();

        let sql_tables = singleton();
        if !sql_tables.contains(&table_name) {
            cx.span_err(path.span, &format!("Table `{}` does not exist", table_name));
        }

        let query = Query::Select{filter: filter, table: table_name};
        return query.to_sql();
    }

    unreachable!();
}

fn expand_sql(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'static> {
    let mut parser = cx.new_parser_from_tts(args);

    let expression = (*parser.parse_expr()).clone();

    match expression.node {
        ExprMethodCall(Spanned { node: method_name, span: method_span}, _, ref arguments) => {
            let method_name = method_name.to_string();

            let this = arguments[0].node.clone();
            let mut arguments = arguments.clone();
            arguments.remove(0);
            let sql = match method_name.as_ref() {
                "collect" => expand_select(cx, this, None),
                "filter" => {
                    let filter = expression_to_filter(&arguments[0], cx);
                    expand_select(cx, this, Some(filter))
                },
                _ => {
                    cx.span_err(method_span, &format!("Unknown method {}", method_name));
                    unreachable!();
                },
            };

            let string_literal = intern(&sql);
            return MacEager::expr(cx.expr_str(sp, InternedString::new_from_name(string_literal)));
        },
        _ => {
            cx.span_err(expression.span, &format!("Expected method call"));
        },
    }

    DummyResult::any(sp)
}

fn expand_sql_table(_: &mut ExtCtxt, _: Span, _: &MetaItem, item: &Annotatable, _: &mut FnMut(Annotatable)) {
    // Add to sql_tables.
    let mut sql_tables = singleton();

    if let &Annotatable::Item(ref item) = item {
        let table_name = item.ident.to_string();
        sql_tables.insert(table_name);
    }
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("sql", expand_sql);
    reg.register_syntax_extension(intern("sql_table"), MultiDecorator(Box::new(expand_sql_table)));
}
