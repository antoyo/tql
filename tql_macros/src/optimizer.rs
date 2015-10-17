/// A Query optimizer.

// TODO: simplifier les expressions composées seulement de litéraux.

use syntax::ast::BinOp_::{BiAdd, BiSub};
use syntax::ast::Expr_::{ExprBinary, ExprLit};
use syntax::ast::Lit_::LitInt;

use ast::{Expression, Limit, Query};
use ast::Limit::{EndRange, Index, LimitOffset, Range, StartRange};
use ast::Query::{CreateTable, Delete, Insert, Select, Update};
use plugin::number_literal;

fn all_integer_literal(expression: &Expression) -> bool {
    match expression.node {
        ExprLit(ref literal) => {
            match literal.node {
                LitInt(_, _) => true,
                _ => false,
            }
        },
        ExprBinary(_, ref expr1, ref expr2) => all_integer_literal(expr1) && all_integer_literal(expr2),
        _ => false,
    }
}

fn evaluate(expression: &Expression) -> u64 {
    match expression.node {
        ExprLit(ref literal) => {
            match literal.node {
                LitInt(number, _) => number,
                _ => 0,
            }
        },
        ExprBinary(op, ref expr1, ref expr2) if op.node == BiAdd => evaluate(expr1) + evaluate(expr2),
        ExprBinary(op, ref expr1, ref expr2) if op.node == BiSub => evaluate(expr1) - evaluate(expr2),
        _ => 0,
    }
}

pub fn optimize(query: &mut Query) {
    match *query {
        CreateTable { .. } => (), // TODO
        Delete { .. } => (), // TODO
        Insert { .. } => (), // TODO
        Select { ref mut limit, .. } => {
            *limit = optimize_limit(limit);
        },
        Update { .. } => (), // TODO
    }
}

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

fn try_simplify(expression: &Expression) -> Expression {
    if all_integer_literal(expression) {
        number_literal(evaluate(expression))
    }
    else {
        expression.clone()
    }
}
