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

/// Analyzer for the sort() method.

use syntax::ast::Expr_::{ExprPath, ExprUnary};
use syntax::ast::UnOp;

use ast::{Expression, Order};
use error::{SqlError, SqlResult, res};
use state::SqlTable;
use super::{check_field, path_expr_to_identifier};

/// Convert an `Expression` to an `Order`.
pub fn argument_to_order(arg: &Expression, table: &SqlTable) -> SqlResult<Order> {
    let mut errors = vec![];
    let order =
        match arg.node {
            ExprUnary(UnOp::UnNeg, ref expr) => {
                let ident = try!(get_identifier(expr, table));
                Order::Descending(ident)
            }
            ExprPath(None, ref path) => {
                let identifier = path.segments[0].identifier.to_string();
                check_field(&identifier, path.span, table, &mut errors);
                Order::Ascending(identifier)
            }
            _ => {
                errors.push(SqlError::new(
                    "Expected - or identifier",
                    arg.span,
                ));
                Order::Ascending("".to_owned())
            }
        };
    res(order, errors)
}

/// Get the `String` indentifying the identifier from an `Expression`.
fn get_identifier(identifier_expr: &Expression, table: &SqlTable) -> SqlResult<String> {
    let mut errors = vec![];
    if let Some(identifier) = path_expr_to_identifier(identifier_expr, &mut errors) {
        check_field(&identifier, identifier_expr.span, table, &mut errors);
        res(identifier, errors)
    }
    else {
        Err(errors)
    }
}
