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

//! Query arguments extractor.

use syn::{
    self,
    Expr,
    Ident,
    parse,
};

use ast::{
    Aggregate,
    AggregateFilterExpression,
    Assignment,
    Expression,
    FilterExpression,
    FilterValue,
    Identifier,
    Limit,
    MethodCall,
    Query,
    query_table,
};
use types::Type;

macro_rules! add_filter_arguments {
    ( $name:ident, $typ:ident, $func:ident ) => {
        /// Create arguments from the `filter` and add them to `arguments`.
        fn $name(filter: $typ, args: &mut Args, literals: &mut Args) {
            match filter {
                $typ::Filter(filter) => {
                    $func(&filter.operand1, args, literals, Some(filter.operand2));
                },
                $typ::Filters(filters) => {
                    $name(*filters.operand1, args, literals);
                    $name(*filters.operand2, args, literals);
                },
                $typ::NegFilter(filter) => {
                    $name(*filter, args, literals);
                },
                $typ::NoFilters => (),
                $typ::ParenFilter(filter) => {
                    $name(*filter, args, literals);
                },
                $typ::FilterValue(filter_value) => {
                    $func(&filter_value.node, args, literals, None);
                },
            }
        }
    };
}

/// A Rust expression to be send as a parameter to the SQL query function.
#[derive(Clone, Debug)]
pub struct Arg {
    pub expression: Expression,
    pub field_name: Option<Identifier>,
}

/// A collection of `Arg`s.
pub type Args = Vec<Arg>;

/// Create an argument from the parameters and add it to `arguments`.
fn add(arguments: &mut Args, literals: &mut Args, field_name: Option<Identifier>, expr: Expression) {
    add_expr(arguments, literals, Arg {
        expression: expr,
        field_name,
    });
}

/// Create arguments from the `assignments` and add them to `arguments`.
fn add_assignments(assignments: Vec<Assignment>, arguments: &mut Args, literals: &mut Args) {
    for assign in assignments {
        let field_name = assign.identifier.expect("Assignment identifier");
        // NOTE: At this stage (code generation), the field exists, hence unwrap().
        add(arguments, literals, Some(field_name.to_string()), assign.value);
    }
}

/// Add an argument to `arguments`.
fn add_expr(arguments: &mut Args, literals: &mut Args, arg: Arg) {
    // Do not add literal.
    if let Expr::Lit(_) = arg.expression {
        literals.push(arg);
        return;
    }
    arguments.push(arg);
}

add_filter_arguments!(add_filter_arguments, FilterExpression, add_filter_value_arguments);

add_filter_arguments!(add_aggregate_filter_arguments, AggregateFilterExpression, add_aggregate_filter_value_arguments);

/// Create arguments from the `limit` and add them to `arguments`.
fn add_limit_arguments(limit: Limit, arguments: &mut Args, literals: &mut Args) {
    match limit {
        Limit::EndRange(expression) => add(arguments, literals, None, expression),
        Limit::Index(expression) => add(arguments, literals, None, expression),
        Limit::LimitOffset(_, _) => (), // NOTE: there are no arguments to add for a `LimitOffset` because it is always using literals.
        Limit::NoLimit => (),
        Limit::Range(expression1, expression2) => {
            let offset = expression1.clone();
            add(arguments, literals, None, expression1);
            let expression = parse((quote! { #expression2 - #offset }).into())
                .expect("Subtraction quoted expression");
            add_expr(arguments, literals, Arg {
                expression,
                field_name: None,
            });
        },
        Limit::StartRange(expression) => add(arguments, literals, None, expression),
    }
}

/// Construct an argument from the method and add it to `args`.
fn add_with_method(args: &mut Args, literals: &mut Args, method_name: &str, object_name: &Ident, index: usize,
                   expr: Expression)
{
    add_expr(args, literals, Arg {
        expression: expr,
        field_name: None,
    });
}

fn add_aggregate_filter_value_arguments(aggregate: &Aggregate, args: &mut Args, literals: &mut Args,
                                        expression: Option<Expression>)
{
    if let Some(expr) = expression {
        add(args, literals, aggregate.field.clone().map(|ident| ident.to_string()), expr); // TODO: use the right type.
    }
}

fn add_filter_value_arguments(filter_value: &FilterValue, args: &mut Args, literals: &mut Args,
                              expression: Option<Expression>)
{
    match *filter_value {
        FilterValue::Identifier(ref table, ref identifier) => {
            // It is possible to have an identifier without expression, when the identifier is a
            // boolean field name, hence this condition.
            if let Some(expr) = expression {
                add(args, literals, Some(format!("{}.{}", table, identifier)), expr);
            }
        },
        FilterValue::MethodCall(MethodCall { ref arguments, ref method_name, ref object_name, .. }) => {
            for (index, arg) in arguments.iter().enumerate() {
                add_with_method(args, literals, method_name, object_name, index, arg.clone());
            }
        },
        FilterValue::None => unreachable!("FilterValue::None in add_filter_value_arguments()"),
    }
}

/// Extract the Rust `Expression`s and the literal arguments from the `Query`.
pub fn arguments(query: Query) -> (Args, Args) {
    let mut arguments = vec![];
    let mut literals = vec![];
    let table_name = query_table(&query);

    match query {
        Query::Aggregate { aggregate_filter, filter, .. } => {
            add_filter_arguments(filter, &mut arguments, &mut literals);
            add_aggregate_filter_arguments(aggregate_filter, &mut arguments, &mut literals);
        },
        Query::CreateTable { .. } => (), // No arguments.
        Query::Delete { filter, .. } => {
            add_filter_arguments(filter, &mut arguments, &mut literals);
        },
        Query::Drop { .. } => (), // No arguments.
        Query::Insert { assignments, .. } => {
            add_assignments(assignments, &mut arguments, &mut literals);
        },
        Query::Select { filter, limit, ..} => {
            add_filter_arguments(filter, &mut arguments, &mut literals);
            add_limit_arguments(limit, &mut arguments, &mut literals);
        },
        Query::Update { assignments, filter, .. } => {
            add_assignments(assignments, &mut arguments, &mut literals);
            add_filter_arguments(filter, &mut arguments, &mut literals);
        },
    }

    (arguments, literals)
}
