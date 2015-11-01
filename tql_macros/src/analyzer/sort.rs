/// Analyzer for the sort() method.

use syntax::ast::{Expr, Path};
use syntax::ast::Expr_::{ExprPath, ExprUnary};
use syntax::ast::UnOp;

use ast::{Expression, Order};
use error::{Error, SqlResult, res};
use state::SqlFields;
use super::check_field;

/// Convert an `Expression` to an `Order`.
pub fn argument_to_order(arg: &Expression, table_name: &str, table: &SqlFields) -> SqlResult<Order> {
    fn identifier(arg: &Expression, identifier: &Expr, table_name: &str, table: &SqlFields) -> SqlResult<String> {
        let mut errors = vec![];
        if let ExprPath(_, Path { ref segments, span, .. }) = identifier.node {
            if segments.len() == 1 {
                let identifier = segments[0].identifier.to_string();
                check_field(&identifier, span, table_name, table, &mut errors);
                return res(identifier, errors);
            }
        }
        Err(vec![Error::new(
            "Expected an identifier".to_owned(),
            arg.span,
        )])
    }

    let mut errors = vec![];
    let order =
        match arg.node {
            ExprUnary(UnOp::UnNeg, ref expr) => {
                let ident = try!(identifier(&arg, expr, table_name, table));
                Order::Descending(ident)
            }
            ExprPath(None, ref path) => {
                let identifier = path.segments[0].identifier.to_string();
                check_field(&identifier, path.span, table_name, table, &mut errors);
                Order::Ascending(identifier)
            }
            _ => {
                errors.push(Error::new(
                    "Expected - or identifier".to_owned(),
                    arg.span,
                ));
                Order::Ascending("".to_owned())
            }
        };
    res(order, errors)
}
