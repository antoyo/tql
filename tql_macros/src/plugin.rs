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
    ExprLit,
    IntSuffix,
    Lit,
    LitInt,
    LitStr,
    parse,
};

pub fn number_literal(num: i64) -> Expr {
    let lit = Expr::Lit(ExprLit {
        attrs: vec![],
        lit: Lit::Int(LitInt::new(num.abs() as u64, IntSuffix::I64, Span::default())),
    });
    if num < 0 {
        parse(quote! {
            -#lit
        }.into()).expect("parse unary minus and lit")
    }
    else {
        lit
    }
}

pub fn string_literal(string: &str) -> Expr {
    Expr::Lit(ExprLit {
        attrs: vec![],
        lit: Lit::Str(LitStr::new(string, Span::default())),
    })
}
