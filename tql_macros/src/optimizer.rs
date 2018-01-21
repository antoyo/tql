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

/// A Query optimizer.

// TODO: simplify expression composed of only literals.

use syn::{
    BinOp,
    Expr,
    ExprLit,
    Lit,
};

use ast::{Expression, Limit, Query};
use ast::Limit::{EndRange, Index, LimitOffset, Range, StartRange};
use plugin::number_literal;

/// Check that all the expressions in `expression` are literal.
fn all_integer_literal(expression: &Expression) -> bool {
    match *expression {
        Expr::Lit(ExprLit { lit: Lit::Int(_), .. }) => true,
        Expr::Binary(ref bin) => all_integer_literal(&bin.left) && all_integer_literal(&bin.right),
        _ => false,
    }
}

/// Reduce an `expression` containing only literals to a mere literal.
fn evaluate(expression: &Expression) -> i64 {
    match *expression {
        Expr::Lit(ref lit) => {
            if let Lit::Int(ref int_literal) = lit.lit {
                // TODO: handle other types.
                int_literal.value() as i64
            }
            else {
                0
            }
        },
        Expr::Binary(ref bin) =>
            match bin.op {
                BinOp::Add(_) => evaluate(&bin.left) + evaluate(&bin.right),
                BinOp::Sub(_) => evaluate(&bin.left) - evaluate(&bin.right),
                _ => 0,
            },
        _ => 0,
    }
}

/// Optimize the query.
pub fn optimize(query: &mut Query) {
    match *query {
        Query::Aggregate { .. } => (), // TODO
        Query::CreateTable { .. } => (), // Nothing to optimize.
        Query::Delete { .. } => (), // TODO
        Query::Drop { .. } => (), // Nothing to optimize.
        Query::Insert { .. } => (), // TODO
        Query::Select { ref mut limit, .. } => {
            *limit = optimize_limit(limit);
        },
        Query::Update { .. } => (), // TODO
    }
}

/// Optimize the limit by simplifying the expressions containing only literal.
fn optimize_limit(limit: &Limit) -> Limit {
    match *limit {
        EndRange(ref expression) => {
            EndRange(try_simplify(expression))
        },
        Index(ref expression) => {
            Index(try_simplify(expression))
        },
        Range(ref expression1, ref expression2) => {
            if all_integer_literal(expression1) && all_integer_literal(expression2) {
                let offset = evaluate(expression1);
                let expr2 = evaluate(expression2);
                let limit = expr2 - offset;
                LimitOffset(number_literal(limit), number_literal(offset))
            }
            else {
                Range(expression1.clone(), expression2.clone())
            }
        },
        StartRange(ref expression) => {
            StartRange(try_simplify(expression))
        },
        ref limit => limit.clone(),
    }
}

/// If `expression` only contains literal, simplify this expression.
/// Otherwise returns it as is.
fn try_simplify(expression: &Expression) -> Expression {
    if all_integer_literal(expression) {
        number_literal(evaluate(expression))
    }
    else {
        expression.clone()
    }
}
