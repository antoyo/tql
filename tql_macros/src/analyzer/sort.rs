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
    Ident,
    UnOp,
};

use ast::{
    Expression,
    Order,
    Query,
    first_token_span,
};
use error::{Error, Result, res};
use super::path_expr_to_identifier;

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
                Order::Ascending(identifier)
            }
            _ => {
                errors.push(Error::new(
                    "Expected - or identifier",
                    first_token_span(arg),
                ));
                Order::NoOrder
            }
        };
    res(order, errors)
}

/// Get the identifier from an `Expression`.
fn get_identifier(identifier_expr: &Expression) -> Result<Ident> {
    let mut errors = vec![];
    if let Some(identifier) = path_expr_to_identifier(identifier_expr, &mut errors) {
        res(identifier, errors)
    }
    else {
        Err(errors)
    }
}

/// Get the identifier in the order by clause to be able to check that they exist.
pub fn get_sort_idents(query: &Query) -> Vec<Ident> {
    let mut idents = vec![];
    if let Query::Select { ref order, ..} = *query {
        for order in order {
            let ident =
                match *order {
                    Order::Ascending(ref ident) => ident,
                    Order::Descending(ref ident) => ident,
                    Order::NoOrder => continue,
                };
            idents.push(ident.clone());
        }
    }
    idents
}
