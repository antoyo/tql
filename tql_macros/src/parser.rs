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

use proc_macro2::Span;
use syn::{
    Expr,
    ExprIndex,
    ExprMethodCall,
    ExprPath,
    Ident,
};
use syn::spanned::Spanned;

use error::{Error, Result, res};

/// A method call.
#[derive(Debug)]
pub struct MethodCall {
    pub args: Vec<Expr>,
    pub name: String,
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
            position: expr.span(),
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
                Expr::MethodCall(ExprMethodCall { ref args, ref receiver, method, .. }) => {
                    add_calls(receiver, calls, errors);

                    let args = args.iter()
                        .cloned()
                        .collect();

                    calls.push(MethodCall {
                        name: method.to_string(),
                        args,
                        position: method.span,
                    });
                },
                Expr::Path(ExprPath { ref path, .. }) => {
                    if path.segments.len() == 1 {
                        calls.name = Some(path.segments.first()
                            .expect("first segment in path").into_item()
                            .ident);
                    }
                },
                Expr::Index(ExprIndex { ref expr, ref index, .. }) => {
                    add_calls(expr, calls, errors);
                    calls.push(MethodCall {
                        name: "limit".to_owned(),
                        args: vec![*index.clone()],
                        position: index.span(),
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
