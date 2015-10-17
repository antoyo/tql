extern crate rustc_front;

use rustc::lint::{LateContext, LateLintPass, LintArray, LintPass};
use rustc::lint::LintContext;
use rustc::middle::ty::{Ty, TypeAndMut, TyS, TypeVariants};
use self::rustc_front::hir::Expr;
use self::rustc_front::hir::Expr_::{self, ExprAddrOf, ExprMethodCall, ExprVec};
use syntax::ast::IntTy::{TyI32, TyI64};
use syntax::codemap::{NO_EXPANSION, BytePos, Span};

use state::{Type, lint_singleton, singleton};
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
    if let ExprAddrOf(_, ref argument) = *arguments {
        if let ExprVec(ref vector) = argument.node {
            for element in vector {
                if let ExprAddrOf(_, ref field) = element.node {
                    types.push(cx.tcx.node_id_to_type(field.id));
                }
            }
        }
    }
    types
}

impl LateLintPass for SqlError {
    fn check_expr(&mut self, cx: &LateContext, expr: &Expr) {
        let tables = singleton();
        if let ExprMethodCall(name, _, ref arguments) = expr.node {
            let method_name = name.node.to_string();
            if method_name == "query" {
                let types = argument_types(cx, &arguments[1].node);
                let calls = lint_singleton();
                let BytePos(low) = expr.span.lo;
                match calls.get(&low) {
                    Some(fields) => {
                        if let Some(table) = tables.get(&fields.table_name) {
                            for (i, typ) in types.iter().enumerate() {
                                let field = &fields.arguments[i];
                                let position = Span {
                                    lo: BytePos(field.low),
                                    hi: BytePos(field.high),
                                    expn_id: NO_EXPANSION,
                                };
                                if field.name == "i64" {
                                    check_type(&Type::I64, typ, position, expr.span, cx);
                                }
                                else if let Some(field_type) = table.get(&field.name) {
                                    check_type(field_type, typ, position, expr.span, cx);
                                }
                                else {
                                    cx.sess().span_err(position, &format!("attempted access of field `{}` on type `{}`, but no field with that name was found", field.name, fields.table_name));
                                    let field_names = fields.arguments.iter().map(|arg| {
                                        &arg.name
                                    });
                                    match find_near(&field.name, field_names) {
                                        Some(name) => {
                                            cx.sess().span_help(position, &format!("did you mean `{}`?", name));
                                        },
                                        None => (),
                                    }
                                }
                            }
                        }
                    },
                    None => (), // TODO
                }
            }
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
