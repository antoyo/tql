/*
 * Copyright (C) 2015  Boucher, Antoni <bouanto@zoho.com>
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

/// Analyzer for the limit() method.

use syntax::ast::Expr;
use syntax::ast::Expr_::{ExprBinary, ExprCall, ExprCast, ExprLit, ExprMethodCall, ExprPath, ExprRange, ExprUnary};
use syntax::ptr::P;

use ast::Limit;
use error::{SqlError, SqlResult, res};
use super::check_type;
use types::Type;

/// Analyze the types of the `Limit`.
pub fn analyze_limit_types(limit: &Limit, errors: &mut Vec<SqlError>) {
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
pub fn argument_to_limit(expression: &P<Expr>) -> SqlResult<Limit> {
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
                errors.push(SqlError::new(
                    "Expected index range or number expression",
                    expression.span,
                ));
                Limit::NoLimit
            }
        };

    // TODO: check if the limit or offset is 0. If this is the case, do not put them in the query
    // (optimization).

    res(limit, errors)
}
