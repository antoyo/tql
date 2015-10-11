extern crate rustc_front;

use rustc::lint::{LateContext, LateLintPass, LintArray, LintPass};
use rustc::lint::LintContext;
use rustc::middle::ty::{Ty, TypeAndMut, TyS, TypeVariants};
use self::rustc_front::hir::Expr;
use self::rustc_front::hir::Expr_::{self, ExprAddrOf, ExprMethodCall, ExprVec};
use syntax::ast::Attribute;
use syntax::ast::IntTy::TyI32;
use syntax::ast::MetaItem_::{MetaList, MetaWord};
use syntax::codemap::{NO_EXPANSION, BytePos, Span};

use state::{SqlArg, Type, lint_singleton, singleton};

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
                                arguments.arguments.push(Some(SqlArg {
                                    // TODO: ne pas utiliser unwrap().
                                    high: args[2].parse().unwrap(),
                                    low: args[1].parse().unwrap(),
                                    name: args[0].clone(),
                                }));
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
        //cx.span_lint(SQL_LINT, expr.span, &format!("{:?}", tables));
        match expr.node {
            ExprMethodCall(name, _, ref arguments) => {
                let method_name = name.node.to_string();
                if method_name == "query" {
                    let types = argument_types(cx, &arguments[1].node);

                    let fields = lint_singleton();
                    // TODO: vérifier les types même quand des litéraux sont utilisés.
                    // TODO: ne pas utiliser unwrap().
                    let table = tables.get(&fields.table_name).unwrap();
                    for i in 0..types.len() {
                        match fields.arguments[i] {
                            Some(ref field) => {
                                let field_type = table.get(&field.name).unwrap();
                                if !same_type(field_type, types[i]) {
                                    let position = Span {
                                        lo: BytePos(field.low),
                                        hi: BytePos(field.high),
                                        expn_id: NO_EXPANSION,
                                    };
                                    cx.span_lint(SQL_LINT, position, &format!("{} should have type {:?}, but have type {:?}", field.name, table.get(&field.name).unwrap(), types[i]));
                                }
                            },
                            None => (),
                        }
                    }
                }
            },
            _ => (),
        }
    }
}

fn same_type(field_type: &Type, expected_type: &TyS) -> bool {
    match expected_type.sty {
        TypeVariants::TyInt(TyI32) => {
            *field_type == Type::Int
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
        _ => false, //panic!(format!("{:?}", expected_type)),
    }
}
