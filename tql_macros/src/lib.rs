//! The TQL library provide macros and attribute useful to generate SQL.
//!
//! The SQL is generated at compile time via a procedural macro.

#![feature(box_syntax, plugin, plugin_registrar, quote, rustc_private)]
#![plugin(clippy)]
#![warn(option_unwrap_used, result_unwrap_used)]

// TODO: paramétriser le type ForeignKey et PrimaryKey pour que la macro puisse choisir de mettre
// le type en question ou rien (dans le cas où la jointure n’est pas faite) ou empêcher les
// modifications (dans le cas où l’ID existe).
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
pub type SqlQueryWithArgs = (String, QueryType, Args, Vec<Join>);

use analyzer::{analyze, analyze_types, has_joins};
use ast::{Expression, FilterExpression, Join, Limit, Query, QueryType, query_type};
use attribute::fields_vec_to_hashmap;
use error::{Error, ErrorType, SqlResult};
use gen::ToSql;
use optimizer::optimize;
use parser::parse;
use state::{SqlArg, SqlArgs, Type, lint_singleton, singleton};
use type_analyzer::SqlError;

/// Extract the Rust `Expression`s from the `Query`.
fn arguments(cx: &mut ExtCtxt, query: Query) -> Args {
    let mut arguments = vec![];

    fn add_expr(arguments: &mut Args, arg: Arg) {
        if let ExprLit(_) = arg.1.node {
            return;
        }
        arguments.push(arg);
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
            Limit::EndRange(expression) => add(arguments, "i64".to_owned(), expression),
            Limit::Index(expression) => add(arguments, "i64".to_owned(), expression),
            Limit::LimitOffset(_, _) => (),
            Limit::NoLimit => (),
            Limit::Range(expression1, expression2) => {
                let offset = expression1.clone();
                add(arguments, "i64".to_owned(), expression1);
                let expr2 = expression2;
                add_expr(arguments, ("i64".to_owned(), quote_expr!(cx, $expr2 - $offset)));
            },
            Limit::StartRange(expression) => add(arguments, "i64".to_owned(), expression),
        }
    }

    match query {
        Query::CreateTable { .. } => (), // TODO
        Query::Delete { .. } => (), // TODO
        Query::Insert { .. } => (), // TODO
        Query::Select {filter, limit, ..} => {
            add_filter_arguments(filter, &mut arguments);
            add_limit_arguments(cx, limit, &mut arguments);
        },
        Query::Update { .. } => (), // TODO
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
            let fields = fields_vec_to_hashmap(struct_def.fields());
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

// TODO: séparer cette fonction en plus petite fonction.
fn gen_query(cx: &mut ExtCtxt, sp: Span, table_ident: Ident, sql_query_with_args: SqlQueryWithArgs) -> Box<MacResult + 'static> {
    // TODO: générer un code différent en fonction du query_type.
    let (sql, query_type, arguments, joins) = sql_query_with_args;
    let string_literal = intern(&sql);
    let string = cx.expr_str(sp, InternedString::new_from_name(string_literal));
    let ident = Ident::new(intern("connection"), table_ident.ctxt);
    // TODO: utiliser un itérateur.
    let sql_tables = singleton();
    let table_name = table_ident.to_string();
    let table = sql_tables.get(&table_name).unwrap();
    let mut fields = vec![];
    // TODO: prendre en compte l’ID.
    let mut index = 0usize;

    for (name, typ) in table {
        match *typ {
            Type::Custom(ref foreign_table) => {
                let table_name = foreign_table;
                match sql_tables.get(foreign_table) {
                    Some(foreign_table) => {
                        if has_joins(&joins, name) {
                            // TODO: seulement aller chercher les champs s’il y a une jointure.
                            let mut foreign_fields = vec![];
                            for (field, typ) in foreign_table {
                                match *typ {
                                    Type::Custom(_) | Type::Dummy => (), // Do not add foreign key recursively.
                                    _ => {
                                        foreign_fields.push(Field {
                                            expr: quote_expr!(cx, row.get($index)),
                                            ident: Spanned {
                                                node: str_to_ident(field),
                                                span: sp,
                                            },
                                            span: sp,
                                        });
                                        index += 1;
                                    },
                                }
                            }
                            let related_struct = cx.expr_struct(sp, cx.path_ident(sp, str_to_ident(table_name)), foreign_fields);
                            fields.push(Field {
                                expr: quote_expr!(cx, Some($related_struct)),
                                ident: Spanned {
                                    node: str_to_ident(name),
                                    span: sp,
                                },
                                span: sp,
                            });
                        }
                        else {
                            fields.push(Field {
                                expr: quote_expr!(cx, None),
                                ident: Spanned {
                                    node: str_to_ident(name),
                                    span: sp,
                                },
                                span: sp,
                            });
                        }
                    },
                    None => (), // Cannot happen.
                }
            },
            Type::Dummy => (),
            _ => {
                fields.push(Field {
                    expr: quote_expr!(cx, row.get($index)),
                    ident: Spanned {
                        node: str_to_ident(name),
                        span: sp,
                    },
                    span: sp,
                });
                index += 1;
            },
        }
    }

    let struct_expr = cx.expr_struct(sp, cx.path_ident(sp, table_ident), fields);

    let mut arg_refs = vec![];
    let mut sql_args = vec![];
    let calls = lint_singleton();

    for (field_name, arg) in arguments {
        let pos = arg.span;

        let (low, high) =
            match (pos.lo, pos.hi) {
                (BytePos(low), BytePos(high)) => (low, high),
            };
        sql_args.push(SqlArg {
            high: high,
            low: low,
            name: field_name,
        });

        match arg.node {
            // Do not add literal arguments as they are in the final string literal.
            ExprLit(_) => (),
            _ => {
                arg_refs.push(cx.expr_addr_of(DUMMY_SP, arg));
            },
        }
    }

    let BytePos(low) = sp.lo;
    calls.insert(low, SqlArgs {
        arguments: sql_args,
        table_name: table_name,
    });

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
    reg.register_late_lint_pass(box SqlError as LateLintPassObject);
}
