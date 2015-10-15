//! Rust compiler plugin functions.

use syntax::ast::Expr;
use syntax::ast::Expr_::ExprLit;
use syntax::ast::Lit_::LitInt;
use syntax::ast::LitIntType::SignedIntLit;
use syntax::ast::IntTy::TyI64;
use syntax::ast::Sign;
use syntax::codemap::{Spanned, DUMMY_SP};
use syntax::ptr::P;

/// Converts a number to an `P<Expr>`.
pub fn number_literal(number: u64) -> P<Expr> {
    P(Expr {
        id: 4294967295,
        node: ExprLit(P(Spanned {
            node: LitInt(number, SignedIntLit(TyI64, Sign::Plus)),
            span: DUMMY_SP,
        })),
        span: DUMMY_SP,
    })
}
