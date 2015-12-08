/*
 * Copyright (C) 2015  Boucher, Antoni <bouanto@zoho.com>
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

//! Rust compiler plugin functions.

use syntax::ast::{Expr, Ident, Path};
use syntax::ast::Expr_::{ExprField, ExprLit};
use syntax::ast::Lit_::LitInt;
use syntax::ast::LitIntType::SignedIntLit;
use syntax::ast::IntTy::TyI64;
use syntax::ast::Sign;
use syntax::codemap::{Span, Spanned, DUMMY_SP};
use syntax::parse::token::intern;
use syntax::ptr::P;

pub static NODE_ID: u32 = 4294967295;

/// Create the `ExprField` expression `expr`.`field_name` (struct field access).
pub fn field_access(expr: P<Expr>, path: &Path, position: Span, field_name: String) -> P<Expr> {
    let syntax_context = path.segments[0].identifier.ctxt;
    let ident = Ident::new(intern(&field_name), syntax_context);
    P(Expr {
        attrs: None,
        id: NODE_ID,
        node: ExprField(expr, Spanned {
            node: ident,
            span: position,
        }),
        span: position,
    })
}

/// Converts a number to an `P<Expr>`.
pub fn number_literal(number: u64) -> P<Expr> {
    P(Expr {
        attrs: None,
        id: NODE_ID,
        node: ExprLit(P(Spanned {
            node: LitInt(number, SignedIntLit(TyI64, Sign::Plus)),
            span: DUMMY_SP,
        })),
        span: DUMMY_SP,
    })
}
