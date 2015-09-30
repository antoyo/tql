//! A module providing functions to convert Rust AST to Sql AST.

use syntax::ast::{BinOp_, Expr, Path};
use syntax::ast::Expr_::{ExprBinary, ExprPath, ExprUnary};
use syntax::codemap::Spanned;
use syntax::ptr::P;

use super::{Filter, FilterExpression, Filters, LogicalOperator, Order, RelationalOperator};
use error::{Error, SqlResult, res};

fn argument_to_order(arg: &Expr) -> SqlResult<Order> {
    fn identifier(arg: &Expr, identifier: &Expr) -> SqlResult<String> {
        if let ExprPath(_, Path { ref segments, .. }) = identifier.node {
            if segments.len() == 1 {
                Ok(segments[0].identifier.to_string())
            }
            else {
                Err(vec![Error {
                    message: "Expected an identifier".to_string(),
                    position: arg.span,
                }])
            }
        }
        else {
            Err(vec![Error {
                message: "Expected an identifier".to_string(),
                position: arg.span,
            }])
        }
    }

    let mut errors = vec![];
    let order =
        match arg.node {
            ExprUnary(_op, ref expr) => {
                // TODO: check if op is -
                let ident = try!(identifier(arg, expr));
                Order::Descending(ident)
            }
            ExprPath(None, ref path) => {
                let ident = path.segments[0].identifier.to_string();
                Order::Ascending(ident)
            }
            _ => {
                errors.push(Error::new(
                    format!("Expected - or identifier"),
                    arg.span,
                ));
                Order::Ascending("".to_string())
            }
        };
    res(order, errors)
}

pub fn arguments_to_orders(arguments: &[P<Expr>]) -> SqlResult<Vec<Order>> {
    let mut orders = vec![];

    for arg in arguments {
        // TODO: conserver toutes les erreurs au lieu d’arrêter à la première.
        let order = try!(argument_to_order(arg));
        orders.push(order);
    }

    Ok(orders)
}

/// Convert a `BinOp_` to an SQL `LogicalOperator`.
pub fn binop_to_logical_operator(binop: BinOp_) -> LogicalOperator {
    match binop {
        BinOp_::BiAdd => unreachable!(),
        BinOp_::BiSub => unreachable!(),
        BinOp_::BiMul => unreachable!(),
        BinOp_::BiDiv => unreachable!(),
        BinOp_::BiRem => unreachable!(),
        BinOp_::BiAnd => LogicalOperator::And,
        BinOp_::BiOr => LogicalOperator::Or,
        BinOp_::BiBitXor => unreachable!(),
        BinOp_::BiBitAnd => unreachable!(),
        BinOp_::BiBitOr => unreachable!(),
        BinOp_::BiShl => unreachable!(),
        BinOp_::BiShr => unreachable!(),
        BinOp_::BiEq => unreachable!(),
        BinOp_::BiLt => unreachable!(),
        BinOp_::BiLe => unreachable!(),
        BinOp_::BiNe => unreachable!(),
        BinOp_::BiGe => unreachable!(),
        BinOp_::BiGt => unreachable!(),
    }
}

/// Convert a `BinOp_` to an SQL `RelationalOperator`.
pub fn binop_to_relational_operator(binop: BinOp_) -> RelationalOperator {
    match binop {
        BinOp_::BiAdd => unreachable!(),
        BinOp_::BiSub => unreachable!(),
        BinOp_::BiMul => unreachable!(),
        BinOp_::BiDiv => unreachable!(),
        BinOp_::BiRem => unreachable!(),
        BinOp_::BiAnd => unreachable!(),
        BinOp_::BiOr => unreachable!(),
        BinOp_::BiBitXor => unreachable!(),
        BinOp_::BiBitAnd => unreachable!(),
        BinOp_::BiBitOr => unreachable!(),
        BinOp_::BiShl => unreachable!(),
        BinOp_::BiShr => unreachable!(),
        BinOp_::BiEq => RelationalOperator::Equal,
        BinOp_::BiLt => RelationalOperator::LesserThan,
        BinOp_::BiLe => RelationalOperator::LesserThanEqual,
        BinOp_::BiNe => RelationalOperator::NotEqual,
        BinOp_::BiGe => RelationalOperator::GreaterThan,
        BinOp_::BiGt => RelationalOperator::GreaterThanEqual,
    }
}

/// Convert a Rust expression to a `FilterExpression`.
pub fn expression_to_filter_expression(arg: &P<Expr>) -> SqlResult<FilterExpression> {
    let mut errors = vec![];
    let dummy = FilterExpression::NoFilters;
    let filter =
        match arg.node {
            ExprBinary(Spanned { node: op, span }, ref expr1, ref expr2) => {
                match expr1.node {
                    ExprPath(None, ref path) => {
                        let identifier = path.segments[0].identifier.to_string();
                        FilterExpression::Filter(Filter {
                            operand1: identifier,
                            operator: binop_to_relational_operator(op),
                            operand2: expr2.clone(),
                        })
                    }
                    ExprBinary(_, _, _) => {
                        let filter1 = try!(expression_to_filter_expression(expr1));
                        let filter2 = try!(expression_to_filter_expression(expr2));
                        FilterExpression::Filters(Filters {
                            operand1: Box::new(filter1),
                            operator: binop_to_logical_operator(op),
                            operand2: Box::new(filter2),
                        })
                    }
                    _ => {
                        errors.push(Error::new(
                            format!("Expected && or ||"),
                            span,
                        ));
                        dummy
                    }
                }
            }
            _ => {
                errors.push(Error::new(
                    format!("Expected binary operation"),
                    arg.span,
                ));
                dummy
            }
        };

    res(filter, errors)
}
