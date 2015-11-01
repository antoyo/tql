//! Query arguments extractor.

use syntax::ast::Expr_::{ExprLit, ExprPath};
use syntax::codemap::Spanned;
use syntax::ext::base::ExtCtxt;

use ast::{Assignment, Expression, FilterExpression, Identifier, Limit, MethodCall, Query, RValue, query_table};
use plugin::field_access;
use state::{SqlMethodTypes, get_primary_key_field, methods_singleton, singleton};
use types::Type;

/// A Rust expression to be send as a parameter to the SQL query function.
#[derive(Clone)]
pub struct Arg {
    pub expression: Expression,
    pub field_name: Option<Identifier>,
    pub typ: Type,
}

/// A collection of `Arg`s.
pub type Args = Vec<Arg>;

/// Create an argument from the parameters and add it to `arguments`.
fn add(arguments: &mut Args, field_name: Option<Identifier>, typ: Type, expr: Expression, table_name: &str) {
    add_expr(arguments, Arg {
        expression: expr,
        field_name: field_name,
        typ: typ,
    }, table_name);
}

/// Create arguments from the `assignments` and add them to `arguments`.
fn add_assignments(assignments: Vec<Assignment>, arguments: &mut Args, table_name: &str) {
    let tables = singleton();
    for assign in assignments {
        if let Some(field_type) = tables.get(table_name).and_then(|table| table.get(&assign.identifier)) {
            add(arguments, Some(assign.identifier), field_type.node.clone(), assign.value, table_name);
        }
    }
}

/// Add an argument to `arguments`.
fn add_expr(arguments: &mut Args, arg: Arg, table_name: &str) {
    let mut new_arg = arg.clone();
    match arg.expression.node {
        ExprLit(_) => return, // Do not add literal.
        ExprPath(_, ref path) => {
            if let Some(ref field_name) = arg.field_name {
                let sql_tables = singleton();
                if let Some(&Spanned { node: Type::Custom(ref related_table_name), .. }) = sql_tables.get(table_name).and_then(|table| table.get(field_name)) {
                    if let Some(table) = sql_tables.get(related_table_name) {
                        if let Some(primary_key_field) = get_primary_key_field(table) {
                            new_arg.expression = field_access(new_arg.expression, path, primary_key_field);
                        }
                    }
                }
            }
        },
        _ => (),
    }
    arguments.push(new_arg);
}

/// Create arguments from the `filter` and add them to `arguments`.
fn add_filter_arguments(filter: FilterExpression, args: &mut Args, table_name: &str) {
    match filter {
        FilterExpression::Filter(filter) => {
            add_rvalue_arguments(&filter.operand1, args, table_name, Some(filter.operand2));
        },
        FilterExpression::Filters(filters) => {
            add_filter_arguments(*filters.operand1, args, table_name);
            add_filter_arguments(*filters.operand2, args, table_name);
        },
        FilterExpression::NegFilter(box filter) => {
            add_filter_arguments(filter, args, table_name);
        },
        FilterExpression::NoFilters => (),
        FilterExpression::ParenFilter(box filter) => {
            add_filter_arguments(filter, args, table_name);
        },
        FilterExpression::RValue(rvalue) => {
            add_rvalue_arguments(&rvalue.node, args, table_name, None);
        },
    }
}

/// Create arguments from the `limit` and add them to `arguments`.
fn add_limit_arguments(cx: &mut ExtCtxt, limit: Limit, arguments: &mut Args, table_name: &str) {
    match limit {
        Limit::EndRange(expression) => add(arguments, None, Type::I64, expression, table_name),
        Limit::Index(expression) => add(arguments, None, Type::I64, expression, table_name),
        Limit::LimitOffset(_, _) => (),
        Limit::NoLimit => (),
        Limit::Range(expression1, expression2) => {
            let offset = expression1.clone();
            add(arguments, None, Type::I64, expression1, table_name);
            let expr2 = expression2;
            add_expr(arguments, Arg {
                expression: quote_expr!(cx, $expr2 - $offset),
                field_name: None,
                typ: Type::I64,
            }, table_name);
        },
        Limit::StartRange(expression) => add(arguments, None, Type::I64, expression, table_name),
    }
}

/// Construct an argument from the method and add it to `args`.
fn add_with_method(args: &mut Args, method_name: &str, object_name: &str, index: usize, expr: Expression, table_name: &str) {
    let tables = singleton();
    let methods = methods_singleton();
    if let Some(field_type) = tables.get(table_name).and_then(|table| table.get(object_name)) {
        if let Some(&SqlMethodTypes { ref argument_types, .. }) = methods.get(&field_type.node).and_then(|type_methods| type_methods.get(method_name)) {
            add_expr(args, Arg {
                expression: expr,
                field_name: None,
                typ: argument_types[index].clone(),
            }, table_name);
        }
    }
}

/// Create arguments from the `rvalue` and add them to `arguments`.
fn add_rvalue_arguments(rvalue: &RValue, args: &mut Args, table_name: &str, expression: Option<Expression>) {
    match *rvalue {
        RValue::Identifier(ref identifier) => {
            if let Some(expr) = expression {
                let tables = singleton();
                if let Some(field_type) = tables.get(table_name).and_then(|table| table.get(identifier)) {
                    add(args, Some(identifier.clone()), field_type.node.clone(), expr, table_name)
                }
            }
        },
        RValue::MethodCall(MethodCall { ref arguments, ref method_name, ref object_name, .. }) => {
            for (index, arg) in arguments.iter().enumerate() {
                add_with_method(args, method_name, object_name, index, arg.clone(), table_name);
            }
        },
    }
}

/// Extract the Rust `Expression`s from the `Query`.
pub fn arguments(cx: &mut ExtCtxt, query: Query) -> Args {
    let mut arguments = vec![];
    let table_name = query_table(&query);

    match query {
        Query::Aggregate { .. } => (), // No arguments.
        Query::CreateTable { .. } => (), // No arguments.
        Query::Delete { filter, .. } => {
            add_filter_arguments(filter, &mut arguments, &table_name);
        },
        Query::Drop { .. } => (), // No arguments.
        Query::Insert { assignments, .. } => {
            add_assignments(assignments, &mut arguments, &table_name);
        },
        Query::Select { filter, limit, ..} => {
            add_filter_arguments(filter, &mut arguments, &table_name);
            add_limit_arguments(cx, limit, &mut arguments, &table_name);
        },
        Query::Update { assignments, filter, .. } => {
            add_filter_arguments(filter, &mut arguments, &table_name);
            add_assignments(assignments, &mut arguments, &table_name);
        },
    }

    arguments
}
