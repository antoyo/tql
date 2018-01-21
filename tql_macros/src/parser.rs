/*
 * Copyright (c) 2017-2018 Boucher, Antoni <bouanto@zoho.com>
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

use proc_macro2::Span;
use syn::{
    Expr,
    Ident,
};
use syn::spanned::Spanned;

use ast::first_token_span;
use error::{Error, Result, res};

/// A method call.
#[derive(Debug)]
pub struct MethodCall {
    pub args: Vec<Expr>,
    pub name: Ident,
    pub position: Span,
}

/// A collection of method calls.
#[derive(Debug)]
pub struct MethodCalls {
    pub calls: Vec<MethodCall>,
    /// The identifier at the start of the calls chain.
    pub name: Option<Ident>,
    pub position: Span,
}

impl MethodCalls {
    fn new(expr: &Expr) -> Self {
        Self {
            calls: vec![],
            name:  None,
            // NOTE: we only want the position of the first token since this position is used in
            // errors for the table name.
            position: first_token_span(expr),
        }
    }

    /// Add a call to the method calls `Vec`.
    fn push(&mut self, call: MethodCall) {
        self.calls.push(call);
    }
}

pub struct Parser {
}

impl Parser {
    pub fn new() -> Self {
        Parser {
        }
    }

    pub fn parse(&self, expr: &Expr) -> Result<MethodCalls> {
        let mut errors = vec![];
        let mut calls = MethodCalls::new(expr);

        /// Add the calls from the `expression` into the `calls` `Vec`.
        fn add_calls(expr: &Expr, calls: &mut MethodCalls, errors: &mut Vec<Error>) {
            match *expr {
                Expr::MethodCall(ref call) => {
                    add_calls(&call.receiver, calls, errors);

                    let args = call.args.iter()
                        .cloned()
                        .collect();

                    calls.push(MethodCall {
                        name: call.method,
                        args,
                        position: expr.span(),
                    });
                },
                Expr::Path(ref path) => {
                    if path.path.segments.len() == 1 {
                        calls.name = Some(path.path.segments.first()
                            .expect("first segment in path").into_value()
                            .ident);
                    }
                },
                Expr::Index(ref index) => {
                    add_calls(&index.expr, calls, errors);
                    calls.push(MethodCall {
                        name: Ident::new("limit", index.index.span()),
                        args: vec![*index.index.clone()],
                        position: expr.span(),
                    });
                }
                _ => {
                    errors.push(Error::new(
                        "Expected method call", // TODO: improve this message.
                        expr.span(),
                    ));
                }
            }
        }

        add_calls(&expr, &mut calls, &mut errors);
        res(calls, errors)
    }
}
