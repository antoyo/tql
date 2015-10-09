/// A Query optimizer.

use syntax::ast::BinOp_::{BiAdd, BiSub};
use syntax::ast::Expr_::{ExprBinary, ExprLit};
use syntax::ast::Lit_::LitInt;

use ast::{Expression, Limit, Query};
use ast::Limit::{LimitOffset, Range};
use ast::Query::Select;

pub fn all_literal(expression: &Expression) -> bool {
    match expression.node {
        ExprLit(_) => true,
        ExprBinary(_, ref expr1, ref expr2) => all_literal(expr1) && all_literal(expr2),
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
        // TODO: simplifier les expressions composées seulement de litéraux.
        // TODO: peut-être que l’optimisation devrait retourner un AST dans un autre format
        // (permettant les litéraux).
        Range(expression1, expression2) => {
            if all_literal(&expression1) && all_literal(&expression2) {
                range_to_limit_offset(&expression1, &expression2)
            }
            else {
                Range(expression1, expression2)
            }
        },
        limit => limit,
    }
}

fn range_to_limit_offset(expression1: &Expression, expression2: &Expression) -> Limit {
    let offset = evaluate(expression1);
    let limit = evaluate(expression2) - offset;
    LimitOffset(limit, offset)
}
