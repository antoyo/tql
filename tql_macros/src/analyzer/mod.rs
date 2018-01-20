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

//! Semantic analyzer.

mod aggregate;
mod assignment;
mod filter;
mod get;
mod insert;
mod method;
mod join;
mod limit;
mod sort;

use std::fmt::Display;
use std::result;

use proc_macro2::Span;
use syn::{
    Expr,
    FloatSuffix,
    Ident,
    IntSuffix,
    Lit,
};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;

use ast::{
    Aggregate,
    AggregateFilterExpression,
    Assignment,
    Expression,
    FilterExpression,
    Groups,
    Join,
    Limit,
    Order,
    Query,
};
use error::{Error, Result, res};
use parser::{MethodCall, MethodCalls};
use plugin::number_literal;
use self::aggregate::{argument_to_aggregate, argument_to_group, expression_to_aggregate_filter_expression};
pub use self::aggregate::{get_aggregate_calls, get_values_idents};
use self::assignment::argument_to_assignment;
use self::filter::{analyze_filter_types, expression_to_filter_expression};
pub use self::filter::get_method_calls;
use self::get::get_expression_to_filter_expression;
use self::insert::check_insert_arguments;
pub use self::insert::get_insert_idents;
#[cfg(feature = "unstable")]
pub use self::insert::get_insert_position;
use self::join::argument_to_join;
use self::limit::{analyze_limit_types, argument_to_limit};
pub use self::limit::get_limit_args;
pub use self::method::analyze_methods;
use self::sort::argument_to_order;
pub use self::sort::get_sort_idents;
use string::{find_near, plural_verb};
use types::Type;

/// The type of the SQL query.
#[derive(PartialEq)]
enum SqlQueryType {
    Aggregate,
    CreateTable,
    Delete,
    Drop,
    Insert,
    Select,
    SelectOne,
    Update,

}

impl Default for SqlQueryType {
    fn default() -> SqlQueryType {
        SqlQueryType::Select
    }
}

/// The query data gathered during the analysis.
#[derive(Default)]
// TODO: improve this design. It should not be necessary to hold data that are not needed for a
// specific query.
struct QueryData {
    // Aggregate
    aggregate_filter: AggregateFilterExpression,
    aggregates: Vec<Aggregate>,
    groups: Groups,
    // Aggregate, Delete, Select, Update
    filter: FilterExpression,
    // Aggregate / Select
    joins: Vec<Join>,
    // Insert / Update
    assignments: Vec<Assignment>,
    // Select
    limit: Limit,
    order: Vec<Order>,
    use_pk: bool,
    // All
    query_type: SqlQueryType,
}

/// Analyze and transform the AST.
pub fn analyze(method_calls: MethodCalls) -> Result<Query> {
    let mut errors = vec![];

    let table_name = method_calls.name.expect("table name in method_calls").to_string();

    check_methods(&method_calls, &mut errors);
    check_method_calls_validity(&method_calls, &mut errors);

    let calls = &method_calls.calls;
    let mut delete_position = None;

    // Get all the data from the query.
    let query_data = process_methods(&calls, &table_name, &mut delete_position)?;

    let query = new_query(query_data, table_name);

    check_delete_without_filters(&query, delete_position, &mut errors);

    res(query, errors)
}

/// Analyze the literal types in the `Query`.
pub fn analyze_types(query: Query) -> Result<Query> {
    let mut errors = vec![];
    match query {
        Query::Aggregate { ref filter, ref table, .. } => {
            analyze_filter_types(filter, &table, &mut errors);
        },
        Query::CreateTable { .. } => (), // Nothing to analyze.
        Query::Delete { ref filter, ref table, .. } => {
            analyze_filter_types(filter, &table, &mut errors);
        },
        Query::Drop { .. } => (), // Nothing to analyze.
        Query::Insert { .. } => (),
        Query::Select { ref filter, ref limit, ref table, .. } => {
            analyze_filter_types(filter, &table, &mut errors);
            analyze_limit_types(limit, &mut errors);
        },
        Query::Update { ref filter, ref table, .. } => {
            analyze_filter_types(filter, &table, &mut errors);
        },
    }
    res(query, errors)
}

/// Check that the `arguments` vector contains `expected_count` elements.
/// If this is not the case, add an error to `errors`.
fn check_argument_count(arguments: &Punctuated<Expr, Comma>, expected_count: usize, position: Span, errors: &mut Vec<Error>) -> bool {
    if arguments.len() == expected_count {
        true
    }
    else {
        let length = arguments.len();
        errors.push(Error::new_with_code(
            &format!("this function takes 1 parameter but {} parameter{} supplied", length, plural_verb(length)),
            position,
            "E0061",
        ));
        false
    }
}

/// Check that `Delete` `Query` contains a filter.
fn check_delete_without_filters(query: &Query, delete_position: Option<Span>, errors: &mut Vec<Error>) {
    if let Query::Delete { ref filter, .. } = *query {
        if let FilterExpression::NoFilters = *filter {
            errors.push(Error::new_warning(
                "delete() without filters",
                delete_position.unwrap(), // There is always a delete position when the query is of type Delete.
            ));
        }
    }
}

/// Check if the method calls sequence is valid.
/// For instance, one cannot call both insert() and delete() methods in the same query.
fn check_method_calls_validity(method_calls: &MethodCalls, errors: &mut Vec<Error>) {
    let method_map =
        hashmap!{
            "aggregate" => vec!["filter", "join", "values"],
            "all" => vec!["filter", "get", "join", "limit", "sort"],
            "create" => vec![],
            "delete" => vec!["filter", "get"],
            "drop" => vec![],
            "insert" => vec![],
            "update" => vec!["filter", "get"],
        };

    let main_method = method_calls.calls.iter()
        .filter(|call| method_map.contains_key(&call.name.as_ref()) )
        .next()
        .map_or("all", |call| call.name.as_ref());

    // TODO: check that the insert, update or delete methods are not called more than once.
    let mut valid_methods = vec![main_method];
    valid_methods.append(&mut method_map[&main_method].clone());

    let methods = get_methods();
    let invalid_methods = method_calls.calls.iter()
        .filter(|call| methods.contains(&call.name.to_string()) && !valid_methods.contains(&call.name.as_ref()));

    for method in invalid_methods {
        errors.push(Error::new(
            &format!("cannot call the {method}() method with the {main_method}() method",
                method = method.name,
                main_method = main_method
            ),
            method.name.span(),
        ));
    }
}

/// Check if the method `calls` exist.
fn check_methods(method_calls: &MethodCalls, errors: &mut Vec<Error>) {
    let methods = get_methods();
    for method_call in &method_calls.calls {
        if !methods.contains(&method_call.name.to_string()) {
            let mut error = Error::new(
                &format!("no method named `{}` found in tql", method_call.name),
                method_call.name.span(),
            );
            propose_similar_name(method_call.name.as_ref(), methods.iter().map(String::as_ref), &mut error);
            errors.push(error);
        }
    }

    if method_calls.calls.is_empty() {
        let table_name = &method_calls.name.expect("table name in method_calls");
        let mut error =
            Error::new_with_code(
                &format!("`{table}` is the name of a struct, but this expression uses it like a method name",
                        table = table_name
                       ),
                method_calls.position, "E0423"
            );
        error.add_help(&format!("did you mean to write `{}.method()`?", table_name));
        errors.push(error);
    }
}

/// Check that the specified method call did not received any arguments.
fn check_no_arguments(method_call: &MethodCall, errors: &mut Vec<Error>) {
    if !method_call.args.is_empty() {
        let length = method_call.args.len();
        errors.push(Error::new_with_code(
            &format!("this method takes 0 parameters but {param_count} parameter{plural} supplied",
                    param_count = length,
                    plural = plural_verb(length)
                   ),
            method_call.name.span(), "E0061"
       ));
    }
}

/// Check if the `field_type` is compatible with the `expression`'s type.
pub fn check_type(field_type: &Type, expression: &Expression, errors: &mut Vec<Error>) {
    if field_type != expression {
        let literal_type = get_type(expression);
        mismatched_types(field_type, &literal_type, expression.span(), errors);
    }
}

/// Convert the `arguments` to the `Type`.
fn convert_arguments<F, Type>(arguments: &[Expression], convert_argument: F) -> Result<Vec<Type>>
    where F: Fn(&Expression) -> Result<Type>
{
    let mut items = vec![];
    let mut errors = vec![];

    for arg in arguments {
        try(convert_argument(arg), &mut errors, |item| {
            items.push(item);
        });
    }

    res(items, errors)
}

/// Get all the existing methods.
// TODO: return Vec<&'static str> instead?
fn get_methods() -> Vec<String> {
    vec![
        "aggregate".to_string(),
        "all".to_string(),
        "create".to_string(),
        "delete".to_string(),
        "drop".to_string(),
        "filter".to_string(),
        "get".to_string(),
        "insert".to_string(),
        "join".to_string(),
        "limit".to_string(),
        "sort".to_string(),
        "update".to_string(),
        "values".to_string(),
    ]
}

/// Get the string representation of an literal `Expression` type.
/// Useful to show in an error.
fn get_type(expression: &Expression) -> &str {
    match *expression {
        Expr::Lit(ref literal) => {
            match literal.lit {
                Lit::Bool(_) => "bool",
                Lit::Int(ref int) =>
                    match int.suffix() {
                        IntSuffix::Isize => "isize",
                        IntSuffix::I8 => "i8",
                        IntSuffix::I128 => "i128",
                        IntSuffix::I16 => "i16",
                        IntSuffix::I32 => "i32",
                        IntSuffix::I64 => "i64",
                        IntSuffix::Usize => "usize",
                        IntSuffix::U8 => "u8",
                        IntSuffix::U128 => "u128",
                        IntSuffix::U16 => "u16",
                        IntSuffix::U32 => "u32",
                        IntSuffix::U64 => "u64",
                        IntSuffix::None => "integral variable",
                    },
                Lit::Byte(_) => "u8",
                Lit::ByteStr(_) => "Vec<u8>",
                Lit::Char(_) => "char",
                Lit::Float(ref float) =>
                    match float.suffix() {
                        FloatSuffix::F32 => "f32",
                        FloatSuffix::F64 => "f64",
                        FloatSuffix::None => "floating-point variable",
                    }
                Lit::Str(_) => "String",
                Lit::Verbatim(_) => panic!("Unsupported integer bigger than 64-bits"),
            }
        }
        _ => panic!("expression needs to be a literal"),
    }
}

/// Add a mismatched types error to `errors`.
fn mismatched_types<S: Display, T: Display>(expected_type: S, actual_type: &T, position: Span, errors: &mut Vec<Error>) {
    let mut error =
        Error::new_with_code(
            &format!("mismatched types:\n expected `{expected_type}`,\n    found `{actual_type}`",
                expected_type = expected_type,
                actual_type = actual_type
            ),
            position,
            "E0308",
        );
    error.add_note("in this expansion of sql! (defined in tql)");
    errors.push(error);
}

/// Create a new query from all the data gathered by the method calls.
fn new_query(QueryData { filter, joins, limit, order, assignments, aggregates, groups,
    aggregate_filter, query_type, use_pk }: QueryData, table_name: String) -> Query
{
    match query_type {
        SqlQueryType::Aggregate =>
            Query::Aggregate {
                aggregates,
                aggregate_filter,
                filter,
                groups,
                joins,
                table: table_name,
            },
        SqlQueryType::CreateTable =>
            Query::CreateTable {
                table: table_name,
            },
        SqlQueryType::Delete =>
            Query::Delete {
                filter,
                table: table_name,
                use_pk,
            },
        SqlQueryType::Drop =>
            Query::Drop {
                table: table_name,
            },
        SqlQueryType::Insert =>
            Query::Insert {
                assignments,
                table: table_name,
            },
        SqlQueryType::Select | SqlQueryType::SelectOne => {
            Query::Select {
                filter,
                get: query_type == SqlQueryType::SelectOne,
                joins,
                limit,
                order,
                table: table_name,
                use_pk,
            }
        },
        SqlQueryType::Update =>
            Query::Update {
                assignments,
                filter,
                table: table_name,
                use_pk,
            },
    }
}

/// Convert an `Expression` to an `Ident` if `expression` is an `ExprPath`.
/// It adds an error to `errors` if `expression` is not an `ExprPath`.
fn path_expr_to_identifier(expression: &Expression, errors: &mut Vec<Error>) -> Option<Ident> {
    if let Expr::Path(ref path) = *expression {
        let identifier = path.path.segments.first().unwrap().into_value().ident.clone();
        Some(identifier)
    }
    else {
        errors.push(Error::new(
            "Expected identifier", // TODO: improve this message.
            expression.span(),
        ));
        None
    }
}

/// Convert an `Expression` to a `String` if `expression` is an `ExprPath`.
/// It adds an error to `errors` if `expression` is not an `ExprPath`.
fn path_expr_to_string(expression: &Expression, errors: &mut Vec<Error>) -> Option<String> {
    path_expr_to_identifier(expression, errors)
        .map(|ident| ident.to_string())
}

/// Gather data about the query in the method `calls`.
/// Also analyze the types.
fn process_methods(calls: &[MethodCall], table_name: &str, delete_position: &mut Option<Span>) -> Result<QueryData> {
    let mut errors = vec![];
    let mut query_data = QueryData::default();

    for method_call in calls {
        match method_call.name.as_ref() {
            "aggregate" => {
                try(convert_arguments(&method_call.args, argument_to_aggregate), &mut errors, |aggrs| {
                    query_data.aggregates = aggrs;
                });
                query_data.query_type = SqlQueryType::Aggregate;
            },
            "all" => {
                check_no_arguments(&method_call, &mut errors);
            },
            "create" => {
                check_no_arguments(&method_call, &mut errors);
                query_data.query_type = SqlQueryType::CreateTable;
            },
            "delete" => {
                check_no_arguments(&method_call, &mut errors);
                query_data.query_type = SqlQueryType::Delete;
                *delete_position = Some(method_call.name.span());
            },
            "drop" => {
                check_no_arguments(&method_call, &mut errors);
                query_data.query_type = SqlQueryType::Drop;
            },
            "filter" => {
                if query_data.aggregates.is_empty() {
                    // If the aggregate() method was not called, filter() filters on the values
                    // (WHERE).
                    try(expression_to_filter_expression(&method_call.args[0], table_name), &mut errors, |filter| {
                        query_data.filter = filter;
                    });
                }
                else {
                    // If the aggregate() method was called, filter() filters on the aggregated
                    // values (HAVING).
                    try(expression_to_aggregate_filter_expression(&method_call.args[0], &query_data.aggregates), &mut errors, |filter| {
                        query_data.aggregate_filter = filter;
                    });
                }
            },
            "get" => {
                if method_call.args.is_empty() {
                    query_data.limit = Limit::Index(number_literal(0));
                }
                else {
                    try(get_expression_to_filter_expression(&method_call.args[0], table_name), &mut errors,
                        |(filter, use_pk, new_limit)| {
                            query_data.filter = filter;
                            query_data.use_pk = use_pk;
                            query_data.limit = new_limit;
                        });
                }
                query_data.query_type = SqlQueryType::SelectOne;
            },
            "insert" => {
                try(convert_arguments(&method_call.args, argument_to_assignment), &mut errors, |assigns| {
                    query_data.assignments = assigns;
                });
                if !query_data.assignments.is_empty() {
                    // TODO: check even if there are errors in the assignation types.
                    check_insert_arguments(&query_data.assignments, &mut errors);
                }
                query_data.query_type = SqlQueryType::Insert;
            },
            "join" => {
                try(convert_arguments(&method_call.args, |expr| argument_to_join(expr, table_name)), &mut errors,
                    |result| {
                        for new_join in result {
                            query_data.joins.push(new_join);
                        }
                    });
            },
            "limit" => {
                try(argument_to_limit(&method_call.args[0]), &mut errors, |new_limit| {
                    query_data.limit = new_limit;
                });
            },
            "sort" => {
                try(convert_arguments(&method_call.args, argument_to_order), &mut errors, |new_order| {
                    query_data.order = new_order;
                });
            },
            "update" => {
                try(convert_arguments(&method_call.args, argument_to_assignment), &mut errors, |assigns| {
                    query_data.assignments = assigns;
                });
                query_data.query_type = SqlQueryType::Update;
            },
            "values" => {
                try(convert_arguments(&method_call.args, argument_to_group), &mut errors, |new_groups| {
                    query_data.groups = new_groups;
                });
            },
            _ => (), // NOTE: Nothing to do since check_methods() check for unknown method.
        }
    }
    res(query_data, errors)
}

/// Check if a name similar to `identifier` exists in `choices` and show a message if one exists.
/// Returns true if a similar name was found.
pub fn propose_similar_name<'a, T>(identifier: &str, choices: T, error: &mut Error) -> bool
    where T: Iterator<Item = &'a str>
{
    if let Some(name) = find_near(&identifier, choices) {
        error.add_help(&format!("did you mean {}?", name));
        true
    }
    else {
        false
    }
}

/// If `result` is an `Err`, add the errors to `errors`.
/// Otherwise, execute the closure.
fn try<F: FnMut(T), T>(mut result: result::Result<T, Vec<Error>>, errors: &mut Vec<Error>, mut fn_using_result: F) {
    match result {
        Ok(value) => fn_using_result(value),
        Err(ref mut errs) => errors.append(errs),
    }
}
