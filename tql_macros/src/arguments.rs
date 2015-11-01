//! Query arguments extractor.

use syntax::ast::Expr_::{ExprLit, ExprPath};
use syntax::ext::base::ExtCtxt;

use ast::{Assignment, Expression, FilterExpression, Identifier, Limit, MethodCall, Query, RValue, query_table};
use plugin::field_access;
use state::{get_field_type, get_method_types, get_primary_key_field_by_table_name};
use types::Type;

/// A Rust expression to be send as a parameter to the SQL query function.
#[derive(Clone, Debug)]
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
    for assign in assignments {
        // NOTE: At this stage (code generation), the field exists, hence unwrap().
        let field_type = get_field_type(table_name, &assign.identifier).unwrap();
        add(arguments, Some(assign.identifier), field_type.clone(), assign.value, table_name);
    }
}

/// Add an argument to `arguments`.
fn add_expr(arguments: &mut Args, arg: Arg, table_name: &str) {
    let mut new_arg = arg.clone();
    match arg.expression.node {
        ExprLit(_) => return, // Do not add literal.
        ExprPath(_, ref path) => {
            // The argument does not have a field name when it is a method call or a limit
            if let Some(ref field_name) = arg.field_name {
                let field_type = get_field_type(table_name, field_name);
                // If a foreign struct is sent as an argument, rewrite it to get its primary key
                // field.
                if let Some(&Type::Custom(ref related_table_name)) = field_type {
                    // NOTE: At this stage (code generation), the primary key exists, hence unwrap().
                    let primary_key_field = get_primary_key_field_by_table_name(related_table_name).unwrap();
                    new_arg.expression = field_access(new_arg.expression, path, primary_key_field);
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
    // NOTE: At this stage (code generation), the method exists, hence unwrap().
    let method_types = get_method_types(table_name, object_name, method_name).unwrap();
    add_expr(args, Arg {
        expression: expr,
        field_name: None,
        typ: method_types.argument_types[index].clone(),
    }, table_name);
}

/// Create arguments from the `rvalue` and add them to `arguments`.
fn add_rvalue_arguments(rvalue: &RValue, args: &mut Args, table_name: &str, expression: Option<Expression>) {
    match *rvalue {
        RValue::Identifier(ref identifier) => {
            // It is possible to have an identifier without expression, when the identifier is a
            // boolean field name, hence this condition.
            if let Some(expr) = expression {
                // NOTE: At this stage (code generation), the field exists, hence unwrap().
                let field_type = get_field_type(table_name, identifier).unwrap();
                add(args, Some(identifier.clone()), field_type.clone(), expr, table_name);
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
