//! A module providing functions to convert Rust AST to Sql AST.

use syntax::ast::{BinOp_, Expr};
use syntax::ast::Expr_::{ExprBinary, ExprPath};
use syntax::codemap::Spanned;
use syntax::ext::base::ExtCtxt;
use syntax::ptr::P;

use super::{Filter, Operator};

/// Convert a `BinOp_` to an SQL `Operator`.
pub fn binop_to_operator(binop: BinOp_) -> Operator {
    match binop {
        BinOp_::BiAdd => unimplemented!(),
        BinOp_::BiSub => unimplemented!(),
        BinOp_::BiMul => unimplemented!(),
        BinOp_::BiDiv => unimplemented!(),
        BinOp_::BiRem => unimplemented!(),
        BinOp_::BiAnd => Operator::And,
        BinOp_::BiOr => Operator::Or,
        BinOp_::BiBitXor => unimplemented!(),
        BinOp_::BiBitAnd => unimplemented!(),
        BinOp_::BiBitOr => unimplemented!(),
        BinOp_::BiShl => unimplemented!(),
        BinOp_::BiShr => unimplemented!(),
        BinOp_::BiEq => Operator::Equal,
        BinOp_::BiLt => Operator::LesserThan,
        BinOp_::BiLe => Operator::LesserThanEqual,
        BinOp_::BiNe => Operator::NotEqual,
        BinOp_::BiGe => Operator::GreaterThan,
        BinOp_::BiGt => Operator::GreaterThanEqual,
    }
}

/// Convert a Rust expression to a `Filter`.
pub fn expression_to_filter(arg: &P<Expr>, cx: &mut ExtCtxt) -> Filter {
    let (binop, identifier, value) =
        match arg.node {
            ExprBinary(Spanned { node: op, .. }, ref expr1, ref expr2) => {
                match expr1.node {
                    ExprPath(None, ref path) => {
                        let identifier = path.segments[0].identifier.to_string();
                        (op, identifier, expr2)
                    },
                    _ => unreachable!()
                }
            },
            _ => {
                cx.span_err(arg.span, &format!("Expected binary operation"));
                unreachable!();
            },
        };

    Filter {
        operand1: identifier,
        operator: binop_to_operator(binop),
        operand2: value.clone(),
    }
}
