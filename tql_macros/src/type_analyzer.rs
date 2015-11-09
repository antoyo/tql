//! Expression type analyzer.

extern crate rustc_front;

use rustc::lint::{EarlyContext, EarlyLintPass, LateContext, LateLintPass, LintArray, LintContext, LintPass};
use rustc::middle::ty::{Ty, TyS};
use self::rustc_front::hir::Expr;
use self::rustc_front::hir::Expr_::{self, ExprAddrOf, ExprMethodCall, ExprVec};
use syntax::ast::Attribute;
use syntax::codemap::{NO_EXPANSION, BytePos, Span};

use analyzer::unknown_table_error;
use error::{Error, ErrorType, SqlResult, res};
use state::{SqlTable, SqlTables, lint_singleton, singleton};
use types::Type;

declare_lint!(SQL_LINT, Forbid, "Err about SQL type errors");
declare_lint!(SQL_ATTR_LINT, Forbid, "Err about SQL table errors");

pub struct SqlError;
pub struct SqlAttrError;

impl LintPass for SqlError {
    fn get_lints(&self) -> LintArray {
        lint_array!(SQL_LINT)
    }
}

impl LintPass for SqlAttrError {
    fn get_lints(&self) -> LintArray {
        lint_array!(SQL_ATTR_LINT)
    }
}

/// Analyze the types of the SQL table struct.
fn analyze_table_types(table: &SqlTable, sql_tables: &SqlTables) -> SqlResult<()> {
    let mut errors = vec![];
    let mut primary_key_count = 0u32;
    for field in table.fields.values() {
        match field.node {
            Type::Custom(ref related_table_name) =>
                if let None = sql_tables.get(related_table_name) {
                    unknown_table_error(related_table_name, field.span, sql_tables, &mut errors);
                },
            Type::Serial => {
                primary_key_count += 1;
            }
            _ => (),
        }
    }
    match primary_key_count {
        0 => errors.insert(0, Error::new_warning("No primary key found".to_owned(), table.position)),
        1 => (), // One primary key is OK.
        _ => errors.insert(0, Error::new_warning("More than one primary key is currently not supported".to_owned(), table.position)),
    }
    res((), errors)
}

/// Get the types of the elements in a `Vec`.
fn argument_types<'a>(cx: &'a LateContext, arguments: &'a Expr_) -> Vec<Ty<'a>> {
    let mut types = vec![];
    if let ExprAddrOf(_, ref argument) = *arguments {
        if let ExprVec(ref vector) = argument.node {
            for element in vector {
                if let ExprAddrOf(_, ref field) = element.node {
                    types.push(cx.tcx.node_id_to_type(field.id));
                }
                else {
                    panic!("Argument should be a `&_`");
                }
            }
        }
        else {
            panic!("Arguments should be a `&Vec<_>`");
        }
    }
    else {
        panic!("Arguments should be a `&Vec<_>`");
    }
    types
}

impl EarlyLintPass for SqlAttrError {
    /// Check the ForeignKey types at the end because the order of the declarations does not matter
    /// in Rust.
    fn exit_lint_attrs(&mut self, cx: &EarlyContext, _: &[Attribute]) {
        static mut analyze_done: bool = false;
        let done = unsafe { analyze_done };
        if !done {
            let sql_tables = singleton();
            for table in sql_tables.values() {
                if let Err(errors) = analyze_table_types(&table, &sql_tables) {
                    span_errors(errors, cx);
                }
            }
        }
        unsafe {
            analyze_done = true;
        }
    }
}

impl LateLintPass for SqlError {
    /// Check the types of the `Vec` argument of the `postgres::stmt::Statement::query` method.
    fn check_expr(&mut self, cx: &LateContext, expr: &Expr) {
        if let ExprMethodCall(name, _, ref arguments) = expr.node {
            let method_name = name.node.to_string();
            if method_name == "query" || method_name == "execute" {
                let types = argument_types(cx, &arguments[1].node);
                let calls = lint_singleton();
                let BytePos(low) = expr.span.lo;
                match calls.get(&low) {
                    Some(fields) => {
                        for (i, typ) in types.iter().enumerate() {
                            let field = &fields.arguments[i];
                            let position = Span {
                                lo: BytePos(field.low),
                                hi: BytePos(field.high),
                                expn_id: NO_EXPANSION,
                            };
                            check_type(&field.typ, typ, position, expr.span, cx);
                        }
                    },
                    None => (), // TODO
                }
            }
        }
    }
}

/// Check that the `field_type` is the same as the `expected_type`.
/// If not, show an error message.
fn check_type(field_type: &Type, expected_type: &TyS, position: Span, note_position: Span, cx: &LateContext) {
    if field_type != expected_type {
        cx.sess().span_err_with_code(position, &format!("mismatched types:\n expected `{}`,    found `{:?}`", field_type, expected_type), "E0308");
        cx.sess().fileline_note(note_position, "in this expansion of sql! (defined in tql)");
    }
}

/// Show the compilation errors.
fn span_errors(errors: Vec<Error>, cx: &EarlyContext) {
    for &Error {ref code, ref message, position, ref kind} in &errors {
        match *kind {
            ErrorType::Error => {
                match *code {
                    Some(ref code) => cx.sess().span_err_with_code(position, &message, code),
                    None => cx.sess().span_err(position, &message),
                }
            },
            ErrorType::Help => {
                cx.sess().fileline_help(position, &message);
            },
            ErrorType::Note => {
                cx.sess().fileline_note(position, &message);
            },
            ErrorType::Warning => {
                cx.sess().span_warn(position, &message);
            },
        }
    }
}
