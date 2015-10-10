//! Semantic analyzer.

use syntax::ast::{BinOp_, Expr, Path};
use syntax::ast::Expr_::{ExprBinary, ExprCall, ExprCast, ExprLit, ExprMethodCall, ExprPath, ExprRange, ExprUnary};
use syntax::ast::UnOp::UnNeg;
use syntax::codemap::Spanned;
use syntax::ptr::P;

use ast::{Filter, FilterExpression, Filters, Limit, LogicalOperator, Order, RelationalOperator, Query};
use ast::Limit::{EndRange, Index, NoLimit, Range, StartRange};
use error::{Error, SqlResult, res};
use parser::MethodCalls;
use state::SqlTables;

/// Analyze and transform the AST.
pub fn analyze<'a, 'b>(method_calls: MethodCalls, sql_tables: &'a SqlTables) -> SqlResult<Query<'b>> {
    // TODO: vérifier que la suite d’appels de méthode est valide.
    // TODO: vérifier que la limite est une variable de type i64.
    let mut errors = vec![];

    let mut filter_expression = FilterExpression::NoFilters;
    let mut order = vec![];
    let mut limit = Limit::NoLimit;

    for method_call in &method_calls.calls {
        if !sql_tables.contains_key(&method_calls.name) {
            errors.push(Error::new(
                format!("Table `{}` does not exist", method_calls.name),
                method_calls.position,
            ));
        }

        match &method_call.name[..] {
            "collect" => (), // TODO
            "filter" => {
                filter_expression = try!(expression_to_filter_expression(&method_call.arguments[0]));
            }
            "limit" => {
                limit = try!(arguments_to_limit(&method_call.arguments[0]));
            }
            "sort" => {
                order = try!(arguments_to_orders(&method_call.arguments));
            }
            _ => {
                errors.push(Error::new(
                    format!("Unknown method {}", method_call.name),
                    method_call.position,
                ));
            }
        }
    }

    let joins = vec![];
    let table_name = method_calls.name.clone();
    let mut fields = vec![];
    match sql_tables.get(&table_name) {
        Some(table) => {
            for field in table.keys() {
                fields.push(field.clone());
            }
        },
        None => (),
    }
    Ok(Query::Select {
        fields: fields,
        filter: filter_expression,
        joins: joins,
        limit: limit,
        order: order,
        table: table_name,
    })
}

fn argument_to_order(arg: &Expr) -> SqlResult<Order> {
    fn identifier(arg: &Expr, identifier: &Expr) -> SqlResult<String> {
        if let ExprPath(_, Path { ref segments, .. }) = identifier.node {
            if segments.len() == 1 {
                return Ok(segments[0].identifier.to_string());
            }
        }
        Err(vec![Error {
            message: "Expected an identifier".to_string(),
            position: arg.span,
        }])
    }

    let mut errors = vec![];
    let order =
        match arg.node {
            ExprUnary(UnNeg, ref expr) => {
                let ident = try!(identifier(arg, expr));
                Order::Descending(ident)
            }
            ExprPath(None, ref path) => {
                let ident = path.segments[0].identifier.to_string();
                Order::Ascending(ident)
            }
            _ => {
                errors.push(Error::new(
                    "Expected - or identifier".to_string(),
                    arg.span,
                ));
                Order::Ascending("".to_string())
            }
        };
    res(order, errors)
}

fn arguments_to_limit(expression: &P<Expr>) -> SqlResult<Limit> {
    let mut errors = vec![];
    let limit =
        match expression.node {
            ExprRange(None, Some(ref range_end)) => {
                Limit::EndRange(range_end.node.clone())
            }
            ExprRange(Some(ref range_start), None) => {
                Limit::StartRange(range_start.node.clone())
            }
            ExprRange(Some(ref range_start), Some(ref range_end)) => {
                // TODO: vérifier que range_start < range_end.
                Limit::Range(range_start.node.clone(), range_end.node.clone())
            }
            ExprLit(_) | ExprPath(_, _) | ExprCall(_, _) | ExprMethodCall(_, _, _) | ExprBinary(_, _, _) | ExprUnary(_, _) | ExprCast(_, _)  => {
                Limit::Index(expression.node.clone())
            }
            _ => {
                errors.push(Error::new(
                    "Expected index range or number expression".to_string(),
                    expression.span,
                ));
                Limit::NoLimit
            }
        };

    // TODO: vérifier si la limite ou le décalage est 0. Le cas échéant, ne pas les mettre dans
    // la requête (optimisation).

    res(limit, errors)
}

fn arguments_to_orders(arguments: &Vec<P<Expr>>) -> SqlResult<Vec<Order>> {
    let mut orders = vec![];
    let mut errors = vec![];

    for arg in arguments {
        match argument_to_order(arg) {
            Ok(order) => orders.push(order),
            Err(ref mut errs) => errors.append(errs),
        }
    }

    res(orders, errors)
}

/// Convert a `BinOp_` to an SQL `LogicalOperator`.
fn binop_to_logical_operator(binop: BinOp_) -> LogicalOperator {
    match binop {
        BinOp_::BiAdd => unreachable!(),
        BinOp_::BiSub => unreachable!(),
        BinOp_::BiMul => unreachable!(),
        BinOp_::BiDiv => unreachable!(),
        BinOp_::BiRem => unreachable!(),
        BinOp_::BiAnd => LogicalOperator::And,
        BinOp_::BiOr => LogicalOperator::Or,
        BinOp_::BiBitXor => unreachable!(),
        BinOp_::BiBitAnd => unreachable!(),
        BinOp_::BiBitOr => unreachable!(),
        BinOp_::BiShl => unreachable!(),
        BinOp_::BiShr => unreachable!(),
        BinOp_::BiEq => unreachable!(),
        BinOp_::BiLt => unreachable!(),
        BinOp_::BiLe => unreachable!(),
        BinOp_::BiNe => unreachable!(),
        BinOp_::BiGe => unreachable!(),
        BinOp_::BiGt => unreachable!(),
    }
}

/// Convert a `BinOp_` to an SQL `RelationalOperator`.
fn binop_to_relational_operator(binop: BinOp_) -> RelationalOperator {
    match binop {
        BinOp_::BiAdd => unreachable!(),
        BinOp_::BiSub => unreachable!(),
        BinOp_::BiMul => unreachable!(),
        BinOp_::BiDiv => unreachable!(),
        BinOp_::BiRem => unreachable!(),
        BinOp_::BiAnd => unreachable!(),
        BinOp_::BiOr => unreachable!(),
        BinOp_::BiBitXor => unreachable!(),
        BinOp_::BiBitAnd => unreachable!(),
        BinOp_::BiBitOr => unreachable!(),
        BinOp_::BiShl => unreachable!(),
        BinOp_::BiShr => unreachable!(),
        BinOp_::BiEq => RelationalOperator::Equal,
        BinOp_::BiLt => RelationalOperator::LesserThan,
        BinOp_::BiLe => RelationalOperator::LesserThanEqual,
        BinOp_::BiNe => RelationalOperator::NotEqual,
        BinOp_::BiGe => RelationalOperator::GreaterThan,
        BinOp_::BiGt => RelationalOperator::GreaterThanEqual,
    }
}

/// Convert a Rust expression to a `FilterExpression`.
fn expression_to_filter_expression(arg: &P<Expr>) -> SqlResult<FilterExpression> {
    let mut errors = vec![];
    let dummy = FilterExpression::NoFilters;
    let filter =
        match arg.node {
            ExprBinary(Spanned { node: op, span }, ref expr1, ref expr2) => {
                match expr1.node {
                    ExprPath(None, ref path) => {
                        let identifier = path.segments[0].identifier.to_string();
                        FilterExpression::Filter(Filter {
                            operand1: identifier,
                            operator: binop_to_relational_operator(op),
                            operand2: expr2.node.clone(),
                        })
                    }
                    ExprBinary(_, _, _) => {
                        let filter1 = try!(expression_to_filter_expression(expr1));
                        let filter2 = try!(expression_to_filter_expression(expr2));
                        FilterExpression::Filters(Filters {
                            operand1: Box::new(filter1),
                            operator: binop_to_logical_operator(op),
                            operand2: Box::new(filter2),
                        })
                    }
                    _ => {
                        errors.push(Error::new(
                            "Expected && or ||".to_string(),
                            span,
                        ));
                        dummy
                    }
                }
            }
            _ => {
                errors.push(Error::new(
                    "Expected binary operation".to_string(),
                    arg.span,
                ));
                dummy
            }
        };

    res(filter, errors)
}
