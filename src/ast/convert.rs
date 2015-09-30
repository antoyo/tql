//! A module providing functions to convert Rust AST to Sql AST.

use syntax::ast::{BinOp_, Expr};
use syntax::ast::Expr_::{ExprBinary, ExprPath};
use syntax::codemap::Spanned;
use syntax::ptr::P;

use super::{Filter, FilterExpression, Filters, LogicalOperator, RelationalOperator};
use error::{Error, SqlResult, res};

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
