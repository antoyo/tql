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

/// Analyzer for the limit() method.

use syn::{ExprKind, ExprRange};

use ast::{Expression, Limit, expr_span};
use error::{Error, Result, res};
use super::check_type;
use types::Type;

/// Analyze the types of the `Limit`.
pub fn analyze_limit_types(limit: &Limit, errors: &mut Vec<Error>) {
    match *limit {
        Limit::EndRange(ref expression) => check_type(&Type::I64, expression, errors),
        Limit::Index(ref expression) => check_type(&Type::I64, expression, errors),
        Limit::LimitOffset(ref expression1, ref expression2) => {
            check_type(&Type::I64, expression1, errors);
            check_type(&Type::I64, expression2, errors);
        },
        Limit::NoLimit => (),
        Limit::Range(ref expression1, ref expression2) => {
            check_type(&Type::I64, expression1, errors);
            check_type(&Type::I64, expression2, errors);
        },
        Limit::StartRange(ref expression) => check_type(&Type::I64, expression, errors),
    }
}

/// Convert an `Expression` to a `Limit`.
pub fn argument_to_limit(expression: &Expression) -> Result<Limit> {
    let mut errors = vec![];
    let limit =
        match expression.node {
            ExprKind::Range(ExprRange { from: None, to: Some(ref range_end), .. }) => {
                Limit::EndRange(*range_end.clone())
            }
            ExprKind::Range(ExprRange { from: Some(ref range_start), to: None, .. }) => {
                Limit::StartRange(*range_start.clone())
            }
            // TODO: check the RangeLimits.
            ExprKind::Range(ExprRange { from: Some(ref range_start), to: Some(ref range_end), .. }) => {
                // TODO: check that range_start < range_end.
                Limit::Range(*range_start.clone(), *range_end.clone())
            }
            ExprKind::Lit(_) | ExprKind::Path(_) | ExprKind::Call(_) | ExprKind::MethodCall(_) |
                ExprKind::Binary(_) | ExprKind::Unary(_) | ExprKind::Cast(_)  => {
                Limit::Index(expression.clone())
            }
            _ => {
                errors.push(Error::new(
                    "Expected index range or number expression",
                    expr_span(expression),
                ));
                Limit::NoLimit
            }
        };

    // TODO: check if the limit or offset is 0. If this is the case, do not put them in the query
    // (optimization).

    res(limit, errors)
}
