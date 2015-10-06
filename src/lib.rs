#![feature(plugin_registrar, quote, rustc_private, slice_patterns)]

// FIXME: unreachable!() fait planter le compilateur.
// FIXME: enlever les clone() inutiles.
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
use syntax::ast::{Field, Ident, MetaItem, TokenTree};
use syntax::ast::Item_::ItemStruct;
use syntax::codemap::{Span, Spanned};
use syntax::ext::base::{Annotatable, DummyResult, ExtCtxt, MacEager, MacResult};
use syntax::ext::base::SyntaxExtension::MultiDecorator;
use syntax::ext::build::AstBuilder;
use syntax::parse::token::{InternedString, Token, intern, str_to_ident};

pub mod ast;
pub mod convert;
pub mod error;
pub mod gen;
pub mod sql;
pub mod state;

use convert::{expression_to_sql, fields_vec_to_hashmap};
use error::{Error, SqlResult};
use state::singleton;

enum QueryType {
    SelectOne,
    SelectMulti,
}

fn to_sql<'a>(cx: &mut ExtCtxt, args: &[TokenTree]) -> SqlResult<(String, QueryType)> {
    let mut parser = cx.new_parser_from_tts(args);
    let expression = (*parser.parse_expr()).clone();
    let sql_tables = singleton();
    let sql_result = try!(expression_to_sql(&expression, sql_tables));
    Ok((sql_result, QueryType::SelectMulti))
}

fn span_errors(errors: Vec<Error>, cx: &mut ExtCtxt) {
    for &Error {ref message, position} in &errors {
        cx.span_err(position, &message);
    }
}

fn expand_to_sql(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'static> {
    let sql_result = to_sql(cx, args);
    match sql_result {
        Ok((sql, _)) => {
            let string_literal = intern(&sql);
            MacEager::expr(cx.expr_str(sp, InternedString::new_from_name(string_literal)))
        }
        Err(errors) => {
            span_errors(errors, cx);
            DummyResult::any(sp)
        }
    }
}

fn gen_query(cx: &mut ExtCtxt, sp: Span, table_ident: Ident, sql: String, query_type: QueryType) -> Box<MacResult + 'static> {
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

    let expr = quote_expr!(cx, {
        let result = $ident.prepare($string).unwrap();
        result.query(&[]).unwrap().iter().map(|row| {
            $struct_expr
        }).collect::<Vec<_>>()
    });

    MacEager::expr(expr)
}

fn expand_sql(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'static> {
    // TODO: si le premier paramètre n’est pas fourni, utiliser "connection".
    if let TokenTree::TtToken(_, Token::Ident(ident, _)) = args[0] {
        let sql_result = to_sql(cx, args);
        match sql_result {
            Ok((sql, query_type)) => {
                gen_query(cx, sp, ident, sql, query_type)
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

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("to_sql", expand_to_sql);
    reg.register_macro("sql", expand_sql);
    reg.register_syntax_extension(intern("sql_table"), MultiDecorator(Box::new(expand_sql_table)));
}

pub fn test() {
    println!("Hello World!");
}
