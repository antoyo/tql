/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

/// Analyzer for the get() method.

use syn::ExprKind;

use ast::{
    Expression,
    Filter,
    FilterExpression,
    FilterValue,
    Limit,
    RelationalOperator,
};
use error::{Result, res};
use plugin::number_literal;
use state::{SqlTable, get_primary_key_field};
use super::no_primary_key;
use super::filter::expression_to_filter_expression;

/// Convert an expression from a `get()` method to a FilterExpression and a Limit.
pub fn get_expression_to_filter_expression(arg: &Expression, table: &SqlTable) -> Result<(FilterExpression, Limit)> {
    let primary_key_field = get_primary_key_field(table);
    match primary_key_field {
        Some(primary_key_field) =>
            match arg.node {
                ExprKind::Lit(_) | ExprKind::Path(_) => {
                    let filter = FilterExpression::Filter(Filter {
                        operand1: FilterValue::Identifier(primary_key_field.clone()),
                        operator: RelationalOperator::Equal,
                        operand2: arg.clone(),
                    });
                    res((filter, Limit::NoLimit), vec![])
                },
                _ => expression_to_filter_expression(arg, table)
                        .and_then(|filter| Ok((filter, Limit::Index(number_literal(0))))),
            },
        None => Err(vec![no_primary_key(&table.name.to_string(), table.position)]),
    }
}
