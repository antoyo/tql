extern crate rustc_front;

use rustc::lint::{LateContext, LateLintPass, LintArray, LintPass};
use rustc::lint::LintContext;
use rustc::middle::ty::{Ty, TypeAndMut, TyS, TypeVariants};
use self::rustc_front::hir::Expr;
use self::rustc_front::hir::Expr_::{self, ExprAddrOf, ExprMethodCall, ExprVec};
use syntax::ast::Attribute;
use syntax::ast::IntTy::{TyI32, TyI64};
use syntax::ast::MetaItem_::{MetaList, MetaWord};
use syntax::codemap::{NO_EXPANSION, BytePos, Span};

use state::{SqlArg, Type, lint_singleton, singleton};
use string::find_near;

declare_lint!(SQL_LINT, Forbid, "Err about SQL type errors");

pub struct SqlError;

impl LintPass for SqlError {
    fn get_lints(&self) -> LintArray {
        lint_array!(SQL_LINT)
    }
}

fn argument_types<'a>(cx: &'a LateContext, arguments: &'a Expr_) -> Vec<Ty<'a>> {
    let mut types = vec![];
    match arguments {
        &ExprAddrOf(_, ref argument) => {
            match argument.node {
                ExprVec(ref vector) => {
                    for element in vector {
                        match element.node {
                            ExprAddrOf(_, ref field) => {
                                types.push(cx.tcx.node_id_to_type(field.id));
                            },
                            _ => (),
                        }
                    }
                },
                _ => (),
            }
        },
        _ => (),
    }
    types
}

impl LateLintPass for SqlError {
    fn check_attribute(&mut self, _: &LateContext, attribute: &Attribute) {
        match attribute.node.value.node {
            MetaList(ref name, ref fields) => {
                if let Some(_) = name.matches("sql_fields").next() {
                    let arguments = lint_singleton();
                    arguments.arguments.clear();
                    for field in fields {
                        match field.node {
                            MetaList(ref name, ref items) => {
                                arguments.table_name = name.to_string();
                                let mut args = vec![];
                                for item in items {
                                    match item.node {
                                        MetaWord(ref arg) => {
                                            args.push(arg.to_string());
                                        },
                                        _ => (),
                                    }
                                }
                                let mut index = 0;
                                while args.len() - index >= 3 {
                                    arguments.arguments.push(Some(SqlArg {
                                        // TODO: ne pas utiliser unwrap().
                                        high: args[index + 2].parse().unwrap(),
                                        low: args[index + 1].parse().unwrap(),
                                        name: args[index].clone(),
                                    }));
                                    index += 3;
                                }
                            },
                            MetaWord(_) => {
                                arguments.arguments.push(None);
                            },
                            _ => (),
                        }
                    }
                }
            },
            _ => (),
        }
    }

    fn check_expr(&mut self, cx: &LateContext, expr: &Expr) {
        let tables = singleton();
        match expr.node {
            ExprMethodCall(name, _, ref arguments) => {
                let method_name = name.node.to_string();
                if method_name == "query" {
                    let types = argument_types(cx, &arguments[1].node);
                    let fields = lint_singleton();

                    if let Some(table) = tables.get(&fields.table_name) {
                        for i in 0..types.len() {
                            match fields.arguments[i] {
                                Some(ref field) => {
                                    let position = Span {
                                        lo: BytePos(field.low),
                                        hi: BytePos(field.high),
                                        expn_id: NO_EXPANSION,
                                    };
                                    if field.name == "i64" {
                                        check_type(&Type::I64, types[i], position, expr.span, cx);
                                    }
                                    else if let Some(field_type) = table.get(&field.name) {
                                        check_type(field_type, types[i], position, expr.span, cx);
                                    }
                                    else {
                                        cx.sess().span_err(position, &format!("attempted access of field `{}` on type `{}`, but no field with that name was found", field.name, fields.table_name));
                                        let field_names = fields.arguments.iter().filter_map(|arg| {
                                            match *arg {
                                                Some(ref arg) => Some(arg.name.clone()),
                                                None => None,
                                            }
                                        }).collect();
                                        match find_near(&field.name, field_names) {
                                            Some(name) => {
                                                cx.sess().span_help(position, &format!("did you mean `{}`?", name));
                                            },
                                            None => (),
                                        }
                                    }
                                },
                                None => (),
                            }
                        }
                    }
                    fields.arguments.clear();
                    fields.table_name = "".to_string();
                }
            },
            _ => (),
        }
    }
}

fn check_type(field_type: &Type, expected_type: &TyS, position: Span, note_position: Span, cx: &LateContext) {
    if !same_type(field_type, expected_type) {
        cx.sess().span_err_with_code(position, &format!("mismatched types:\n expected `{}`,    found `{:?}`", field_type, expected_type), "E0308");
        cx.sess().fileline_note(note_position, "in this expansion of sql! (defined in tql)");
    }
}

fn same_type(field_type: &Type, expected_type: &TyS) -> bool {
    match expected_type.sty {
        TypeVariants::TyInt(TyI32) => {
            *field_type == Type::I32
        },
        TypeVariants::TyInt(TyI64) => {
            *field_type == Type::I64
        },
        TypeVariants::TyRef(_, TypeAndMut { ty, .. }) => {
            // TODO: supporter les références de références.
            match ty.sty {
                TypeVariants::TyStr => {
                    *field_type == Type::String
                },
                _ => false,
            }
        },
        _ => false,
    }
}
