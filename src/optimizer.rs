/// A Query optimizer.

// TODO: simplifier les expressions composées seulement de litéraux.

use syntax::ast::BinOp_::{BiAdd, BiSub};
use syntax::ast::Expr_::{ExprBinary, ExprLit};
use syntax::ast::Lit_::LitInt;

use ast::{Expression, Limit, Query};
use ast::Limit::{EndRange, Index, LimitOffset, Range, StartRange};
use ast::Query::Select;
use plugin::number_literal;

pub fn all_literal(expression: &Expression) -> bool {
    match expression {
        &ExprLit(_) => true,
        &ExprBinary(_, ref expr1, ref expr2) => all_literal(&expr1.node) && all_literal(&expr2.node),
        _ => false,
    }
}

fn evaluate(expression: &Expression) -> u64 {
    match *expression {
        ExprLit(ref literal) => {
            match literal.node {
                LitInt(number, _) => number,
                _ => 0,
            }
        },
        ExprBinary(op, ref expr1, ref expr2) if op.node == BiAdd => evaluate(&expr1.node) + evaluate(&expr2.node),
        ExprBinary(op, ref expr1, ref expr2) if op.node == BiSub => evaluate(&expr1.node) - evaluate(&expr2.node),
        _ => 0,
    }
}

pub fn optimize(query: Query) -> Query {
    match query {
        // TODO: utiliser la syntaxe de mise à jour.
        Select { fields, filter, joins, limit, order, table } => Select {
            fields: fields,
            filter: filter,
            joins: joins,
            limit: optimize_limit(limit),
            order: order,
            table: table,
        },
        _ => query,
    }
}

fn optimize_limit(limit: Limit) -> Limit {
    match limit {
        EndRange(ref expression) => {
            EndRange(try_simplify(expression))
        },
        Index(ref expression) => {
            Index(try_simplify(expression))
        },
        Range(ref expression1, ref expression2) => {
            if all_literal(expression1) && all_literal(expression2) {
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
        limit => limit,
    }
}

fn try_simplify(expression: &Expression) -> Expression {
    if all_literal(expression) {
        number_literal(evaluate(expression))
    }
    else {
        expression.clone()
    }
}
