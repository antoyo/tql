#![feature(plugin_registrar, rustc_private, slice_patterns)]

// FIXME: unreachable!() fait planter le compilateur.
// FIXME: enlever les clone() inutiles.
// TODO: supporter plusieurs SGBDs.
// TODO: faire des benchmarks.
// TODO: créer une macro qui permet de choisir le SGBD. Donner un paramètre optionel à cette macro
// pour choisir le nom de la macro à créer (pour permettre d’utiliser plusieurs SGBDs à la fois).
// TODO: utiliser une compilation en 2 passes pour détecter les champs utilisés et les jointures
// utiles (peut-être possible avec un lint plugin).

extern crate rustc;
extern crate syntax;

use std::collections::HashSet;
use std::mem;

use rustc::plugin::Registry;
use syntax::ast::{MetaItem, TokenTree};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt, MacEager, MacResult};
use syntax::ext::base::SyntaxExtension::MultiDecorator;
use syntax::ext::build::AstBuilder;
use syntax::parse::token::{InternedString, intern};

pub mod ast;
pub mod convert;
pub mod gen;

use convert::{SqlTables, expression_to_sql};

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

fn expand_sql(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'static> {
    let mut parser = cx.new_parser_from_tts(args);
    let expression = (*parser.parse_expr()).clone();
    let sql_tables = singleton();
    let sql = expression_to_sql(cx, &expression, sql_tables);
    let string_literal = intern(&sql);
    MacEager::expr(cx.expr_str(sp, InternedString::new_from_name(string_literal)))
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
