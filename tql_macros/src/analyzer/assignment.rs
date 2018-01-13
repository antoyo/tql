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

/// Argument to assignment converter.

use proc_macro2::Span;
use syn::{
    BinOp,
    Expr,
};
use syn::spanned::Spanned;

use ast::{
    Assignment,
    AssignementOperator,
    Expression,
    WithSpan,
};
use error::{Error, Result, res};
use plugin::number_literal;
use super::path_expr_to_identifier;

/// Convert an `Expression` to an `Assignment`.
pub fn argument_to_assignment(arg: &Expression) -> Result<Assignment> {
    fn assign_values(assignment: &mut Assignment, expr1: &Expression, expr2: &Expression, errors: &mut Vec<Error>) {
        assignment.value = expr2.clone();
        if let Some(identifier) = path_expr_to_identifier(expr1, errors) {
            assignment.identifier = Some(identifier);
        }
    }

    let mut errors = vec![];
    let mut assignment = Assignment {
        identifier: None,
        operator: WithSpan {
            node: AssignementOperator::Equal,
            span: arg.span(),
        },
        value: number_literal(0),
    };
    match *arg {
        Expr::Assign(ref oper) => {
            assign_values(&mut assignment, &oper.left, &oper.right, &mut errors);
        },
        Expr::AssignOp(ref oper) => {
            let (node, span) = binop_to_assignment_operator(&oper.op);
            assignment.operator = WithSpan {
                node,
                span,
            };
            assign_values(&mut assignment, &oper.left, &oper.right, &mut errors);
        },
        _ => {
            errors.push(Error::new(
                "Expected assignment", // TODO: improve this message.
                arg.span(),
            ));
        },
    }
    res(assignment, errors)
}

/// Convert a `BinOp` to an SQL `AssignmentOperator`.
fn binop_to_assignment_operator(binop: &BinOp) -> (AssignementOperator, Span) {
    match *binop {
        BinOp::AddEq(span) => (AssignementOperator::Add, span.0[0]),
        BinOp::SubEq(span) => (AssignementOperator::Sub, span.0[0]),
        BinOp::MulEq(span) => (AssignementOperator::Mul, span.0[0]),
        BinOp::DivEq(span) => (AssignementOperator::Divide, span.0[0]),
        BinOp::RemEq(span) => (AssignementOperator::Modulo, span.0[0]),
        BinOp::Eq(span) => (AssignementOperator::Equal, span.0[0]),
        BinOp::Add(_) | BinOp::Sub(_) | BinOp::Mul(_) | BinOp::Div(_) | BinOp::Rem(_) | BinOp::And(_) |
            BinOp::Or(_) | BinOp::BitXor(_) | BinOp::BitXorEq(_) | BinOp::BitAnd(_) | BinOp::BitAndEq(_) |
            BinOp::BitOr(_) | BinOp::BitOrEq(_) | BinOp::Shl(_) | BinOp::ShlEq(_) | BinOp::Shr(_) | BinOp::ShrEq(_) |
            BinOp::Lt(_) | BinOp::Le(_) | BinOp::Ne(_) | BinOp::Ge(_) | BinOp::Gt(_) =>
                unreachable!("binop_to_assignment_operator"),
    }
}
