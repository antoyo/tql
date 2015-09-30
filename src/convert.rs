use std::collections::HashSet;

use syntax::ast::Expr;
use syntax::ast::Expr_::{ExprMethodCall, ExprPath};
use syntax::codemap::{Span, Spanned};
use syntax::ptr::P;

use ast::{Fields, FilterExpression, Query};
use ast::convert::expression_to_filter_expression;
use gen::ToSql;

use error::{Error, SqlResult, res};

// TODO: mettre ce type dans lib.rs.
pub type SqlTables = HashSet<String>;

#[derive(Debug)]
pub struct MethodCall<'a> {
    pub arguments: &'a [P<Expr>],
    pub name: String,
    pub position: Span,
}

#[derive(Debug)]
pub struct MethodCalls<'a> {
    pub calls: Vec<MethodCall<'a>>,
    pub name: String,
    pub position: Span,
}

impl<'a> MethodCalls<'a> {
    fn push(&mut self, call: MethodCall<'a>) {
        self.calls.push(call);
    }
}

fn method_calls_to_sql(method_calls: &MethodCalls, sql_tables: &SqlTables) -> SqlResult<String> {
    // TODO: prendre en compte tous les éléments du vecteur.
    let method_call = &method_calls.calls[0];
    let mut errors = vec![];

    if !sql_tables.contains(&method_calls.name) {
        errors.push(Error::new(
            format!("Table `{}` does not exist", method_calls.name),
            method_calls.position,
        ));
    }

    let filter_expression = match &method_call.name[..] {
        "collect" => FilterExpression::NoFilters,
        "filter" => {
            try!(expression_to_filter_expression(&method_call.arguments[0]))
        },
        _ => {
            errors.push(Error::new(
                format!("Unknown method {}", method_call.name),
                method_call.position,
            ));
            FilterExpression::NoFilters
        },
    };

    let joins = vec![];
    let limit = None;
    let order = vec![];

    let query = Query::Select {
        fields: Fields::All,
        filter: filter_expression,
        joins: &joins,
        limit: limit,
        order: &order,
        table: method_calls.name.clone(),
    };
    res(query.to_sql(), errors)
}

pub fn expression_to_sql(expression: &Expr, sql_tables: &SqlTables) -> SqlResult<String> {
    let method_calls = try!(expression_to_vec(&expression));
    method_calls_to_sql(&method_calls, sql_tables)
}

/// Convert a method call expression to a simpler vector-based structure.
fn expression_to_vec<'a>(expression: &'a Expr) -> SqlResult<MethodCalls<'a>> {
    let mut errors = vec![];
    let mut calls = MethodCalls {
        calls: vec![],
        name:  "".to_string(),
        position: expression.span,
    };

    fn expr_to_vec<'a>(expression: &'a Expr, calls: &mut MethodCalls<'a>, errors: &mut Vec<Error>) {
        match expression.node {
            ExprMethodCall(Spanned { node: object, span: method_span}, _, ref arguments) => {
                expr_to_vec(&arguments[0], calls, errors);

                calls.push(MethodCall {
                    arguments: &arguments[1..],
                    name: object.to_string(),
                    position: method_span,
                });
            },
            ExprPath(_, ref path) => {
                if path.segments.len() == 1 {
                    calls.name = path.segments[0].identifier.to_string();
                }
            },
            // TODO: indexation (Table[0..10]).
            _ => {
                errors.push(Error::new(
                    format!("Expected method call"),
                    expression.span,
                ));
            },
        }
    }

    expr_to_vec(expression, &mut calls, &mut errors);
    res(calls, errors)
}
