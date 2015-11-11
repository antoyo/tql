//! Query arguments extractor.

use syntax::ast::Expr_::ExprLit;
use syntax::ext::base::ExtCtxt;

use ast::{Assignment, Expression, FilterExpression, FilterValue, Identifier, Limit, MethodCall, Query, query_table};
use state::{get_field_type, get_method_types};
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
fn add(arguments: &mut Args, field_name: Option<Identifier>, typ: Type, expr: Expression) {
    add_expr(arguments, Arg {
        expression: expr,
        field_name: field_name,
        typ: typ,
    });
}

/// Create arguments from the `assignments` and add them to `arguments`.
fn add_assignments(assignments: Vec<Assignment>, arguments: &mut Args, table_name: &str) {
    for assign in assignments {
        // NOTE: At this stage (code generation), the field exists, hence unwrap().
        let field_type = get_field_type(table_name, &assign.identifier).unwrap();
        add(arguments, Some(assign.identifier), field_type.clone(), assign.value);
    }
}

/// Add an argument to `arguments`.
fn add_expr(arguments: &mut Args, arg: Arg) {
    // Do not add literal.
    if let ExprLit(_) = arg.expression.node {
        return;
    }
    arguments.push(arg);
}

/// Create arguments from the `filter` and add them to `arguments`.
fn add_filter_arguments(filter: FilterExpression, args: &mut Args, table_name: &str) {
    match filter {
        FilterExpression::Filter(filter) => {
            add_filter_value_arguments(&filter.operand1, args, table_name, Some(filter.operand2));
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
        FilterExpression::FilterValue(filter_value) => {
            add_filter_value_arguments(&filter_value.node, args, table_name, None);
        },
    }
}

/// Create arguments from the `limit` and add them to `arguments`.
fn add_limit_arguments(cx: &mut ExtCtxt, limit: Limit, arguments: &mut Args) {
    match limit {
        Limit::EndRange(expression) => add(arguments, None, Type::I64, expression),
        Limit::Index(expression) => add(arguments, None, Type::I64, expression),
        Limit::LimitOffset(_, _) => (),
        Limit::NoLimit => (),
        Limit::Range(expression1, expression2) => {
            let offset = expression1.clone();
            add(arguments, None, Type::I64, expression1);
            let expr2 = expression2;
            add_expr(arguments, Arg {
                expression: quote_expr!(cx, $expr2 - $offset),
                field_name: None,
                typ: Type::I64,
            });
        },
        Limit::StartRange(expression) => add(arguments, None, Type::I64, expression),
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
    });
}

/// Create arguments from the `filter_value` and add them to `arguments`.
fn add_filter_value_arguments(filter_value: &FilterValue, args: &mut Args, table_name: &str, expression: Option<Expression>) {
    match *filter_value {
        FilterValue::Identifier(ref identifier) => {
            // It is possible to have an identifier without expression, when the identifier is a
            // boolean field name, hence this condition.
            if let Some(expr) = expression {
                // NOTE: At this stage (code generation), the field exists, hence unwrap().
                let field_type = get_field_type(table_name, identifier).unwrap();
                add(args, Some(identifier.clone()), field_type.clone(), expr);
            }
        },
        FilterValue::MethodCall(MethodCall { ref arguments, ref method_name, ref object_name, .. }) => {
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
            add_limit_arguments(cx, limit, &mut arguments);
        },
        Query::Update { assignments, filter, .. } => {
            add_assignments(assignments, &mut arguments, &table_name);
            add_filter_arguments(filter, &mut arguments, &table_name);
        },
    }

    arguments
}
