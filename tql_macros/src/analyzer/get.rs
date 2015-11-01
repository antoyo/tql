/// Analyzer for the get() method.

use syntax::ast::Expr;
use syntax::ast::Expr_::{ExprLit, ExprPath};
use syntax::ptr::P;

use ast::{Filter, FilterExpression, Limit, RelationalOperator, RValue};
use error::{SqlResult, res};
use plugin::number_literal;
use state::{SqlFields, get_primary_key_field};
use super::no_primary_key;
use super::filter::expression_to_filter_expression;

/// Convert an expression from a `get()` method to a FilterExpression and a Limit.
pub fn get_expression_to_filter_expression(arg: &P<Expr>, table_name: &str, table: &SqlFields) -> SqlResult<(FilterExpression, Limit)> {
    let primary_key_field = get_primary_key_field(table);
    match primary_key_field {
        Some(primary_key_field) =>
            match arg.node {
                ExprLit(_) | ExprPath(_, _) => {
                    let filter = FilterExpression::Filter(Filter {
                        operand1: RValue::Identifier(primary_key_field),
                        operator: RelationalOperator::Equal,
                        operand2: arg.clone(),
                    });
                    res((filter, Limit::NoLimit), vec![])
                },
                _ => expression_to_filter_expression(arg, table_name, table)
                        .and_then(|filter| Ok((filter, Limit::Index(number_literal(0))))),
            },
        None => Err(vec![no_primary_key(table_name, arg.span)]), // TODO: utiliser la vraie position.
    }
}
