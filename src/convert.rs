use std::collections::HashSet;

use syntax::ast::Expr;
use syntax::ast::Expr_::{ExprMethodCall, ExprPath};
use syntax::codemap::{Span, Spanned};
use syntax::ext::base::ExtCtxt;
use syntax::ptr::P;

use ast::{Fields, FilterExpression, Query};
use ast::convert::expression_to_filter_expression;
use gen::ToSql;

// TODO: mettre ce type dans lib.rs.
pub type SqlTables = HashSet<String>;

#[derive(Debug)]
pub struct MethodCall<'a> {
    pub arguments: &'a [P<Expr>], // TODO: utiliser un slice pour supprimer le premier élement.
    pub name: String,
    pub position: Span,
}

#[derive(Debug)]
pub struct MethodCalls<'a> {
    pub calls: Vec<MethodCall<'a>>,
    pub name: String,
}

impl<'a> MethodCalls<'a> {
    fn push(&mut self, call: MethodCall<'a>) {
        self.calls.push(call);
    }
}

fn method_calls_to_sql(cx: &mut ExtCtxt, method_calls: &MethodCalls, sql_tables: &SqlTables) -> String {
    // TODO: prendre en compte tous les éléments du vecteur.
    let method_call = &method_calls.calls[0];

    let filter_expression = match &method_call.name[..] {
        "collect" => FilterExpression::NoFilters,
        "filter" => {
            expression_to_filter_expression(&method_call.arguments[0], cx)
        },
        _ => {
            cx.span_err(method_call.position, &format!("Unknown method {}", method_call.name));
            unreachable!();
        },
    };

    if !sql_tables.contains(&method_calls.name) {
        cx.span_err(method_call.position, &format!("Table `{}` does not exist", method_calls.name));
    }

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
    return query.to_sql();
}

// TODO: retourner un Result et spanner l’erreur dans lib.rs.
pub fn expression_to_sql(cx: &mut ExtCtxt, expression: &Expr, sql_tables: &SqlTables) -> String {
    let method_calls = expression_to_vec(cx, &expression);
    method_calls_to_sql(cx, &method_calls, sql_tables)
}

fn expr_to_vec<'a>(cx: &mut ExtCtxt, expression: &'a Expr, calls: &mut MethodCalls<'a>) {
    match expression.node {
        ExprMethodCall(Spanned { node: object, span: method_span}, _, ref arguments) => {
            expr_to_vec(cx, &arguments[0], calls);

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
            cx.span_err(expression.span, &format!("Expected method call"));
        },
    }
}

/// Convert a method call expression to a simpler vector-based structure.
fn expression_to_vec<'a>(cx: &mut ExtCtxt, expression: &'a Expr) -> MethodCalls<'a> {
    let mut calls = MethodCalls {
        calls: vec![],
        name:  "".to_string(),
    };
    expr_to_vec(cx, expression, &mut calls);
    return calls;
}
