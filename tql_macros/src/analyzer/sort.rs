/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

/// Analyzer for the sort() method.

use syn::{
    Expr,
    ExprUnary,
    UnOp,
};
use syn::spanned::Spanned;

use ast::{Expression, Order};
use error::{Error, Result, res};
use super::{check_field, path_expr_to_identifier};

/// Convert an `Expression` to an `Order`.
pub fn argument_to_order(arg: &Expression) -> Result<Order> {
    let mut errors = vec![];
    let order =
        match *arg {
            Expr::Unary(ExprUnary { op: UnOp::Neg(_), ref expr, .. }) => {
                let ident = get_identifier(expr)?;
                Order::Descending(ident)
            }
            Expr::Path(ref path) => {
                let identifier = path.path.segments.first().unwrap().into_value().ident;
                check_field(&identifier, identifier.span, &mut errors);
                Order::Ascending(identifier.to_string())
            }
            _ => {
                errors.push(Error::new(
                    "Expected - or identifier",
                    arg.span(),
                ));
                Order::Ascending("".to_string())
            }
        };
    res(order, errors)
}

/// Get the `String` indentifying the identifier from an `Expression`.
fn get_identifier(identifier_expr: &Expression) -> Result<String> {
    let mut errors = vec![];
    if let Some(identifier) = path_expr_to_identifier(identifier_expr, &mut errors) {
        // TODO: check that the field is in the struct.
        //check_field(&identifier, identifier_expr.span(), table, &mut errors);
        res(identifier.to_string(), errors)
    }
    else {
        Err(errors)
    }
}
