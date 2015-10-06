use std::collections::HashMap;

use syntax::ast::{Expr, Path, StructField_, StructFieldKind, Ty};
use syntax::ast::Expr_::{ExprMethodCall, ExprPath};
use syntax::ast::Ty_::TyPath;
use syntax::codemap::{Span, Spanned};
use syntax::ptr::P;

use ast::{Fields, FilterExpression, Query};
use ast::convert::{arguments_to_orders, expression_to_filter_expression};
use gen::ToSql;

use error::{Error, SqlResult, res};
use state::{SqlFields, SqlTables, Type};

#[derive(Debug)]
struct MethodCall<'a> {
    arguments: &'a [P<Expr>],
    name: String,
    position: Span,
}

#[derive(Debug)]
struct MethodCalls<'a> {
    calls: Vec<MethodCall<'a>>,
    name: String,
    position: Span,
}

impl<'a> MethodCalls<'a> {
    fn push(&mut self, call: MethodCall<'a>) {
        self.calls.push(call);
    }
}

fn method_calls_to_sql(method_calls: &MethodCalls, sql_tables: &SqlTables) -> SqlResult<String> {
    // TODO: vérifier que la suite d’appels de méthode est valide.
    let mut errors = vec![];

    let mut filter_expression = FilterExpression::NoFilters;
    let mut order = vec![];

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
            "sort" => {
                order = try!(arguments_to_orders(method_call.arguments));
            }
            _ => {
                errors.push(Error::new(
                    format!("Unknown method {}", method_call.name),
                    method_call.position,
                ));
            }
        };
    }

    let joins = vec![];
    let limit = None;
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

/// Convert a Rust expression to SQL.
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

fn field_ty_to_type(ty: &Ty) -> Type {
    let mut typ = Type::Dummy;
    if let TyPath(None, Path { ref segments, .. }) = ty.node {
        if segments.len() == 1 {
            let ident = segments[0].identifier.to_string();
            if ident == "String" {
                typ = Type::String
            }
            else if ident == "i32" {
                typ = Type::Int

            }
        }
    }
    typ
}

pub fn fields_vec_to_hashmap(fields: &Vec<Spanned<StructField_>>) -> SqlFields {
    let mut sql_fields = HashMap::new();
    // TODO: ajouter le champ id.
    //sql_fields.insert("id".to_string(), Type::Int);
    for field in fields {
        if let StructFieldKind::NamedField(ident, _) = field.node.kind {
            sql_fields.insert(ident.to_string(), field_ty_to_type(&*field.node.ty));
        }
    }
    sql_fields
}
