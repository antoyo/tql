//! Rust compiler plugin functions.

use syntax::ast::Expr;
use syntax::ast::Expr_::{self, ExprLit};
use syntax::ast::Lit_::LitInt;
use syntax::ast::LitIntType::SignedIntLit;
use syntax::ast::IntTy::TyI64;
use syntax::ast::Sign;
use syntax::codemap::{Spanned, DUMMY_SP};
use syntax::ptr::P;

/// Converts a number to an `Expr_`.
pub fn number_literal(number: u64) -> Expr_ {
    ExprLit(P(Spanned {
        node: LitInt(number, SignedIntLit(TyI64, Sign::Plus)),
        span: DUMMY_SP,
    }))
}

/// Convert an `Expression` to a `P<Expr>`.
pub fn to_expr(expr: Expr_) -> P<Expr> {
    P(Expr {
        id: 4294967295,
        node: expr,
        span: DUMMY_SP,
    })
}
