//! The TQL library provide macros and attribute useful to generate SQL.
//!
//! The SQL is generated at compile time via a procedural macro.

#![feature(box_syntax, plugin_registrar, quote, rustc_private)]

// FIXME: unreachable!() fait planter le compilateur.
// FIXME: remplacer format!() par .to_string() quand c’est possible.
// FIXME: enlever les clone() inutiles.
// FIXME: utiliser des fermetures à la place de fonctions internes.
// FIXME: utiliser use self au lieu de deux lignes.
// TODO: créer différents types pour String.
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

#[macro_use]
extern crate rustc;
extern crate syntax;

use rustc::lint::LateLintPassObject;
use rustc::plugin::Registry;
use syntax::ast::{Expr, Field, Ident, MetaItem, TokenTree};
use syntax::ast::Expr_::ExprLit;
use syntax::ast::Item_::ItemStruct;
use syntax::codemap::{DUMMY_SP, BytePos, Span, Spanned};
use syntax::ext::base::{Annotatable, DummyResult, ExtCtxt, MacEager, MacResult};
use syntax::ext::base::SyntaxExtension::MultiDecorator;
use syntax::ext::build::AstBuilder;
use syntax::feature_gate::AttributeType::Whitelisted;
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
pub mod string;
pub mod type_analyzer;

type Arg = (String, P<Expr>);
type Args = Vec<Arg>;
pub type SqlQueryWithArgs = (String, QueryType, Args);

use analyzer::{analyze, analyze_types};
use ast::{Expression, FilterExpression, Limit, Query, QueryType, query_type};
use attribute::fields_vec_to_hashmap;
use error::{Error, ErrorType, SqlResult};
use gen::ToSql;
use optimizer::optimize;
use parser::parse;
use state::singleton;
use type_analyzer::SqlError;

/// Extract the Rust `Expression`s from the `Query`.
fn arguments(cx: &mut ExtCtxt, query: Query) -> Args {
    let mut arguments = vec![];

    fn add_expr(arguments: &mut Args, arg: Arg) {
        let (_, expression) = arg.clone();
        match expression.node {
            ExprLit(_) => (),
            _ => arguments.push(arg),
        }
    }

    fn add(arguments: &mut Args, field_name: String, expr: Expression) {
        let arg = (field_name, expr);
        add_expr(arguments, arg);
    }

    fn add_filter_arguments(filter: FilterExpression, arguments: &mut Args) {
        match filter {
            FilterExpression::Filter(filter) => {
                add(arguments, filter.operand1, filter.operand2);
            },
            FilterExpression::Filters(filters) => {
                add_filter_arguments(*filters.operand1, arguments);
                add_filter_arguments(*filters.operand2, arguments);
            },
            FilterExpression::NoFilters => (),
        }
    }

    fn add_limit_arguments(cx: &mut ExtCtxt, limit: Limit, arguments: &mut Args) {
        match limit {
            Limit::EndRange(expression) => add(arguments, "i64".to_string(), expression),
            Limit::Index(expression) => add(arguments, "i64".to_string(), expression),
            Limit::LimitOffset(_, _) => (),
            Limit::NoLimit => (),
            Limit::Range(expression1, expression2) => {
                let offset = expression1.clone();
                add(arguments, "i64".to_string(), expression1);
                let expr2 = expression2;
                let offset = offset;
                add_expr(arguments, ("i64".to_string(), quote_expr!(cx, $expr2 - $offset)));
            },
            Limit::StartRange(expression) => add(arguments, "i64".to_string(), expression),
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
            // TODO: vérifier le type des champs de la structure.
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
    let mut meta_field_words = vec![];
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

    for (field_name, arg) in arguments {
        let pos = arg.span;

        let (low, high) =
            match (pos.lo, pos.hi) {
                (BytePos(low), BytePos(high)) => (low, high),
            };
        let low = InternedString::new_from_name(str_to_ident(&low.to_string()).name);
        let high = InternedString::new_from_name(str_to_ident(&high.to_string()).name);
        let list = vec![str_to_ident(&field_name).name.as_str(), low, high];
        for el in list {
            meta_field_words.push(cx.meta_word(DUMMY_SP, el));
        }

        match arg.node {
            // Do not add literal arguments as they are in the final string literal.
            ExprLit(_) => (),
            _ => {
                arg_refs.push(cx.expr_addr_of(DUMMY_SP, arg));
            },
        }
    }
    let args_expr = cx.expr_vec(DUMMY_SP, arg_refs);
    let meta_list = cx.meta_list(DUMMY_SP, InternedString::new_from_name(table_ident.name), meta_field_words);
    let sql_fields_attribute = cx.meta_list(DUMMY_SP, InternedString::new("sql_fields"), vec![meta_list]);

    let expr = match query_type {
        QueryType::SelectMulti => {
            quote_expr!(cx, {
                #[$sql_fields_attribute]
                #[allow(dead_code)]
                const FIELD: i32 = 3141592;
                let result = $ident.prepare($string).unwrap();
                result.query(&$args_expr).unwrap().iter().map(|row| {
                    $struct_expr
                }).collect::<Vec<_>>()
            })
        },
        QueryType::SelectOne => {
            quote_expr!(cx, {
                #[$sql_fields_attribute]
                #[allow(dead_code)]
                const FIELD: i32 = 3141592;
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
    for &Error {code, ref message, position, ref typ} in &errors {
        match *typ {
            ErrorType::Error => {
                match code {
                    Some(code) => cx.parse_sess.span_diagnostic.span_err_with_code(position, &message, code),
                    None => cx.span_err(position, &message),
                }
            },
            ErrorType::Help => {
                cx.parse_sess.span_diagnostic.fileline_help(position, &message);
            },
            ErrorType::Note => {
                cx.parse_sess.span_diagnostic.fileline_note(position, &message);
            },
        }
    }
}

fn to_sql<'a>(cx: &mut ExtCtxt, args: &[TokenTree]) -> SqlResult<'a, SqlQueryWithArgs> {
    let mut parser = cx.new_parser_from_tts(args);
    let expression = (*parser.parse_expr()).clone();
    let sql_tables = singleton();
    let method_calls = try!(parse(expression));
    let mut query = try!(analyze(method_calls, sql_tables));
    optimize(&mut query);
    query = try!(analyze_types(query));
    let sql = query.to_sql();
    Ok((sql, query_type(&query), arguments(cx, query)))
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("to_sql", expand_to_sql);
    reg.register_macro("sql", expand_sql);
    reg.register_attribute("sql_fields".to_string(), Whitelisted);
    reg.register_syntax_extension(intern("sql_table"), MultiDecorator(Box::new(expand_sql_table)));
    reg.register_late_lint_pass(box SqlError as LateLintPassObject);
}
