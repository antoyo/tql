//! Workaround to make get the hygiene right on stable.

use quote::Tokens;
use syn::{
    Expr,
    Ident,
};

use ast::{
    Aggregate,
    Assignment,
    AssignmentOperator,
    FilterExpression,
    FilterValue,
    Groups,
    Join,
    Limit,
    LogicalOperator,
    MethodCall,
    Order,
    Query,
    RelationalOperator,
};
use parser::MethodCalls;

pub fn generate_macro_patterns(query: &Query, calls: &MethodCalls) -> Tokens {
    let mut count = 0;
    let mut dummy_count = 0;
    let mut args = vec![];
    let table_name = calls.name.expect("table name");
    let mut methods = quote! {};
    for call in &calls.calls {
        let name = call.name;
        let args =
            match name.as_ref() {
                "all" | "create" | "delete" | "drop" => quote! {},
                "aggregate" =>
                    if let Query::Aggregate { ref aggregates, .. } = *query {
                        aggregates_to_args(aggregates)
                    }
                    else {
                        quote! {}
                    },
                "filter" | "get" =>
                    match *query {
                        Query::Aggregate { ref filter, .. } | Query::Delete { ref filter, .. } |
                            Query::Select { ref filter, .. } | Query::Update { ref filter, .. } =>
                            filter_to_args(filter, &mut dummy_count, &mut count, &mut args),
                        _ => quote! {},
                    },
                "join" =>
                    match *query {
                        Query::Aggregate { ref joins, .. } | Query::Select { ref joins, .. } =>
                            joins_to_args(joins),
                        _ => quote! {},
                    }
                "insert" | "update" =>
                    match *query {
                        Query::Insert { ref assignments, .. } | Query::Update { ref assignments, .. } =>
                            assignments_to_args(assignments, &mut dummy_count, &mut count, &mut args),
                        _ => quote! {},
                    },
                "limit" =>
                    if let Query::Select { ref limit, .. } = *query {
                        limit_to_args(limit, &mut dummy_count, &mut count, &mut args)
                    }
                    else {
                        quote! {}
                    },
                "sort" =>
                    if let Query::Select { ref order, .. } = *query {
                        order_to_args(order)
                    }
                    else {
                        quote! {}
                    },
                "values" =>
                    if let Query::Aggregate { ref groups, .. } = *query {
                        values_to_args(groups)
                    }
                    else {
                        quote! {}
                    },
                _ => unreachable!("No method named {}", name),
            };
        methods =
            if name == "limit" {
                quote! {
                    #methods
                    #args
                }
            }
            else {
                quote! {
                    #methods
                    . #name (#args)
                }
            };
    }
    let args =
        if args.len() == 1 {
            quote! { #(&$#args)* }
        }
        else {
            quote! { (#(&$#args),*) }
        };
    quote! {
        #[allow(unused)]
        macro_rules! __tql_extract_exprs {
            (#table_name #methods) => {
                #args
            };
        }
    }
}

fn filter_to_args(filter: &FilterExpression, dummy_count: &mut i32, count: &mut i32, args: &mut Vec<Ident>) -> Tokens {
    match *filter {
        FilterExpression::Filter(ref filter) => {
            let left = filter_value_to_args(&filter.operand1);
            let op =
                if left == quote! {} {
                    quote! {}
                }
                else {
                    rel_op_to_args(filter.operator)
                };
            let right = &filter.operand2;
            let right = expr_to_args(right, dummy_count, count, args);
            quote! {
                #left #op #right
            }
        },
        FilterExpression::Filters(ref filters) => {
            let left = filter_to_args(&filters.operand1, dummy_count, count, args);
            let op = log_op_to_args(filters.operator);
            let right = filter_to_args(&filters.operand2, dummy_count, count, args);
            quote! {
                #left #op #right
            }
        },
        FilterExpression::FilterValue(ref value) => filter_value_to_args(&value.node),
        FilterExpression::NegFilter(ref filter) => {
            let expr = filter_to_args(filter, dummy_count, count, args);
            quote! { - #expr }
        },
        FilterExpression::NoFilters => quote! {},
        FilterExpression::ParenFilter(ref filter) => {
            let expr = filter_to_args(filter, dummy_count, count, args);
            quote! { ( #expr ) }
        },
    }
}

fn filter_value_to_args(filter_value: &FilterValue) -> Tokens {
    match *filter_value {
        FilterValue::Identifier(_, ref identifier) => {
            quote! { #identifier }
        },
        FilterValue::MethodCall(MethodCall { ref arguments, ref method_name, ref object_name, .. }) => quote! {
            #object_name . #method_name ( #(#arguments),* )
        },
        FilterValue::None => unreachable!(),
        FilterValue::PrimaryKey(_) => quote! { },
    }
}

fn log_op_to_args(operator: LogicalOperator) -> Tokens {
    match operator {
        LogicalOperator::And => quote! { && },
        LogicalOperator::Not => quote! { ! },
        LogicalOperator::Or => quote! { || },
    }
}

fn rel_op_to_args(operator: RelationalOperator) -> Tokens {
    match operator {
        RelationalOperator::Equal => quote! { == },
        RelationalOperator::LesserThan => quote! { < },
        RelationalOperator::LesserThanEqual => quote! { <= },
        RelationalOperator::NotEqual => quote! { != },
        RelationalOperator::GreaterThan => quote! { > },
        RelationalOperator::GreaterThanEqual => quote! { >= },
    }
}

fn assign_op_to_args(operator: AssignmentOperator) -> Tokens {
    match operator {
        AssignmentOperator::Add => quote! { += },
        AssignmentOperator::Divide => quote! { /= },
        AssignmentOperator::Equal => quote! { = },
        AssignmentOperator::Modulo => quote! { %= },
        AssignmentOperator::Mul => quote! { *= },
        AssignmentOperator::Sub => quote! { -= },
    }
}

fn assignments_to_args(assignments: &Vec<Assignment>, dummy_count: &mut i32, count: &mut i32, args: &mut Vec<Ident>) -> Tokens {
    let assignments = assignments.iter()
        .map(|assignment| {
            let ident = &assignment.identifier;
            let op = assign_op_to_args(assignment.operator.node);
            let expr = expr_to_args(&assignment.value, dummy_count, count, args);
            quote! {
                #ident #op #expr
            }
        });
    quote! {
        #(#assignments),*
    }
}

fn order_to_args(order: &[Order]) -> Tokens {
    let orders =
        order.iter()
            .map(|order|
                 match *order {
                     Order::Ascending(ref ident) => quote! { #ident },
                     Order::Descending(ref ident) => quote! { - #ident },
                     Order::NoOrder => quote! {},
                 }
            );
    quote! {
        #(#orders),*
    }
}

fn limit_to_args(limit: &Limit, dummy_count: &mut i32, count: &mut i32, args: &mut Vec<Ident>) -> Tokens {
    match *limit {
        Limit::EndRange(ref expr) => {
            let expr = expr_to_args(expr, dummy_count, count, args);
            quote! { [.. #expr] }
        },
        Limit::Index(ref expr) => {
            let expr = expr_to_args(expr, dummy_count, count, args);
            quote! { [#expr] }
        },
        Limit::NoLimit => quote! { },
        Limit::LimitOffset(ref expr1, ref expr2) | Limit::Range(ref expr1, ref expr2) => {
            let expr1 = expr_to_args(expr1, dummy_count, count, args);
            let expr2 = expr_to_args(expr2, dummy_count, count, args);
            quote! { [#expr1 .. #expr2] }
        },
        Limit::StartRange(ref expr) => {
            let expr = expr_to_args(expr, dummy_count, count, args);
            quote! { [#expr .. ] }
        },
    }
}

fn expr_to_args(expr: &Expr, dummy_count: &mut i32, count: &mut i32, args: &mut Vec<Ident>) -> Tokens {
    match *expr {
        Expr::Lit(_) => {
            *dummy_count += 1;
            let ident = Ident::from(format!("__tql_dummy_arg{}", *dummy_count));
            quote! {
                $#ident : tt
            }
        },
        _ => {
            *count += 1;
            let ident = Ident::from(format!("__tql_arg{}", *count));
            args.push(ident.clone());
            quote! {
                $#ident : ident
            }
        },
    }
}

fn joins_to_args(joins: &[Join]) -> Tokens {
    let joins = joins.iter()
        .map(|join| {
            let base_field = &join.base_field;
            let base_table = Ident::from(join.base_table.as_str());
            quote! {
                #base_table.#base_field
            }
        });
    quote! {
        #(#joins),*
    }
}

fn aggregates_to_args(aggregates: &[Aggregate]) -> Tokens {
    let aggregates = aggregates.iter()
        .map(|aggregate| {
            let result =
                if aggregate.has_name_in_query {
                    let name = aggregate.result_name.expect("result name");
                    quote! { #name = }
                }
                else {
                    quote! {}
                };
            let function = &aggregate.function;
            let field = &aggregate.field;
            quote! {
                #result #function(#field)
            }
        });
    quote! {
        #(#aggregates),*
    }
}

fn values_to_args(groups: &Groups) -> Tokens {
    quote! {
        #(#groups),*
    }
}
