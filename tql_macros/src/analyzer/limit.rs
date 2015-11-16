/// Analyzer for the limit() method.

use syntax::ast::Expr;
use syntax::ast::Expr_::{ExprBinary, ExprCall, ExprCast, ExprLit, ExprMethodCall, ExprPath, ExprRange, ExprUnary};
use syntax::ptr::P;

use ast::Limit;
use error::{Error, SqlResult, res};
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

/// Convert a slice of `Expression` to a `Limit`.
pub fn arguments_to_limit(expression: &P<Expr>) -> SqlResult<Limit> {
    let mut errors = vec![];
    let limit =
        match expression.node {
            ExprRange(None, Some(ref range_end)) => {
                Limit::EndRange(range_end.clone())
            }
            ExprRange(Some(ref range_start), None) => {
                Limit::StartRange(range_start.clone())
            }
            ExprRange(Some(ref range_start), Some(ref range_end)) => {
                // TODO: check that range_start < range_end.
                Limit::Range(range_start.clone(), range_end.clone())
            }
            ExprLit(_) | ExprPath(_, _) | ExprCall(_, _) | ExprMethodCall(_, _, _) | ExprBinary(_, _, _) | ExprUnary(_, _) | ExprCast(_, _)  => {
                Limit::Index(expression.clone())
            }
            _ => {
                errors.push(Error::new(
                    "Expected index range or number expression".to_owned(),
                    expression.span,
                ));
                Limit::NoLimit
            }
        };

    // TODO: check if the limit or offset is 0. If this is the case, do not put them in the query
    // (optimization).

    res(limit, errors)
}
