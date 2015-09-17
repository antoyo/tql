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
use syntax::ast::{BinOp_, Expr, Expr_, Item, MetaItem, TokenTree};
use syntax::ast::Expr_::{ExprBinary, ExprMethodCall, ExprPath};
use syntax::codemap::{Span, Spanned};
use syntax::ext::base::{Annotatable, DummyResult, ExtCtxt, MacEager, MacResult, MultiItemDecorator};
use syntax::ext::base::SyntaxExtension::MultiDecorator;
use syntax::ext::build::AstBuilder;
use syntax::parse::token::{InternedString, intern};
use syntax::ptr::P;

use std::collections::HashSet;
use std::mem;

type SqlTables = HashSet<String>;

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

#[derive(Debug)]
enum Operator {
    And,
    Or,
    Eq,
    Lt,
    Le,
    Ne,
    Ge,
    Gt,
}

impl ToString for Operator {
    fn to_string(&self) -> String {
        match *self {
            Operator::And => "AND".to_string(),
            Operator::Or => "OR".to_string(),
            Operator::Eq => "=".to_string(),
            Operator::Lt => "<".to_string(),
            Operator::Le => "<=".to_string(),
            Operator::Ne => "<>".to_string(),
            Operator::Ge => ">=".to_string(),
            Operator::Gt => ">".to_string(),
        }
    }
}

#[derive(Debug)]
struct Filter {
    identifier: String,
    operator: Operator,
    value: P<Expr>,
}

impl ToString for Filter {
    fn to_string(&self) -> String {
        self.identifier.clone() + " " + &self.operator.to_string() + " ?"
    }
}

fn expand_select(cx: &mut ExtCtxt, expr: Expr_, filter: Option<Filter>) -> String {
    if let ExprPath(None, path) = expr {
        let table_name = path.segments[0].identifier.to_string();

        let sql_tables = singleton();
        if !sql_tables.contains(&table_name) {
            cx.span_err(path.span, &format!("Table `{}` does not exist", table_name));
        }

        let mut where_clause = String::new();

        if let Some(filter) = filter {
            where_clause.push_str(" WHERE ");
            where_clause.push_str(&filter.to_string());
        }

        return format!("SELECT * FROM {}{}", table_name, where_clause);
    }

    unreachable!();
}

fn binop_to_operator(binop: BinOp_) -> Operator {
    match binop {
        BinOp_::BiAdd => unimplemented!(),
        BinOp_::BiSub => unimplemented!(),
        BinOp_::BiMul => unimplemented!(),
        BinOp_::BiDiv => unimplemented!(),
        BinOp_::BiRem => unimplemented!(),
        BinOp_::BiAnd => Operator::And,
        BinOp_::BiOr => Operator::Or,
        BinOp_::BiBitXor => unimplemented!(),
        BinOp_::BiBitAnd => unimplemented!(),
        BinOp_::BiBitOr => unimplemented!(),
        BinOp_::BiShl => unimplemented!(),
        BinOp_::BiShr => unimplemented!(),
        BinOp_::BiEq => Operator::Eq,
        BinOp_::BiLt => Operator::Lt,
        BinOp_::BiLe => Operator::Le,
        BinOp_::BiNe => Operator::Ne,
        BinOp_::BiGe => Operator::Ge,
        BinOp_::BiGt => Operator::Gt,
    }
}

fn arg_to_filter(arg: &P<Expr>, cx: &mut ExtCtxt) -> Filter {
    let (operator, identifier, value) =
        match arg.node {
            ExprBinary(Spanned { node: op, .. }, ref expr1, ref expr2) => {
                match expr1.node {
                    ExprPath(None, ref path) => {
                        let identifier = path.segments[0].identifier.to_string();
                        (binop_to_operator(op), identifier, expr2)
                    },
                    _ => unreachable!()
                }
            },
            _ => {
                cx.span_err(arg.span, &format!("Expected binary operation"));
                unreachable!();
            },
        };

    Filter {
        identifier: identifier,
        operator: operator,
        value: value.clone(),
    }
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
                    let filter = arg_to_filter(&arguments[0], cx);
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

fn expand_sql_table(cx: &mut ExtCtxt, sp: Span, meta_item: &MetaItem, item: &Annotatable, push: &mut FnMut(Annotatable)) {
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
