/// Analyzer for the get() method.

use syntax::ast::Expr;
use syntax::ast::Expr_::{ExprLit, ExprPath};
use syntax::ptr::P;

use ast::{Filter, FilterExpression, FilterValue, Limit, RelationalOperator};
use error::{SqlResult, res};
use plugin::number_literal;
use state::{SqlTable, get_primary_key_field};
use super::no_primary_key;
use super::filter::expression_to_filter_expression;

/// Convert an expression from a `get()` method to a FilterExpression and a Limit.
pub fn get_expression_to_filter_expression(arg: &P<Expr>, table: &SqlTable) -> SqlResult<(FilterExpression, Limit)> {
    let primary_key_field = get_primary_key_field(table);
    match primary_key_field {
        Some(primary_key_field) =>
            match arg.node {
                ExprLit(_) | ExprPath(_, _) => {
                    let filter = FilterExpression::Filter(Filter {
                        operand1: FilterValue::Identifier(primary_key_field),
                        operator: RelationalOperator::Equal,
                        operand2: arg.clone(),
                    });
                    res((filter, Limit::NoLimit), vec![])
                },
                _ => expression_to_filter_expression(arg, table)
                        .and_then(|filter| Ok((filter, Limit::Index(number_literal(0))))),
            },
        None => Err(vec![no_primary_key(&table.name, table.position)]),
    }
}
