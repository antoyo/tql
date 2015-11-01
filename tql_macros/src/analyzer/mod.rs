//! Semantic analyzer.

use std::borrow::Cow;
use std::fmt::Display;

use syntax::ast::Expr;
use syntax::ast::Expr_::ExprLit;
use syntax::ast::FloatTy;
use syntax::ast::IntTy;
use syntax::ast::Lit_::{LitBool, LitByte, LitByteStr, LitChar, LitFloat, LitFloatUnsuffixed, LitInt, LitStr};
use syntax::ast::LitIntType::{SignedIntLit, UnsignedIntLit, UnsuffixedIntLit};
use syntax::ast::UintTy;
use syntax::codemap::{Span, Spanned};
use syntax::ptr::P;

mod aggregate;
mod assignment;
mod filter;
mod get;
mod insert;
mod join;
mod limit;
mod sort;

use ast::{self, Aggregate, Assignment, Expression, FilterExpression, Identifier, Join, Limit, Order, RValue, Query, TypedField};
use error::{Error, SqlResult, res};
use gen::ToSql;
use parser::{MethodCall, MethodCalls};
use self::aggregate::argument_to_aggregate;
use self::assignment::{analyze_assignments_types, argument_to_assignment};
use self::filter::{analyze_filter_types, expression_to_filter_expression};
use self::get::get_expression_to_filter_expression;
use self::insert::check_insert_arguments;
use self::join::argument_to_join;
use self::limit::{analyze_limit_types, arguments_to_limit};
use self::sort::argument_to_order;
use state::{SqlFields, SqlTables, get_field_type, methods_singleton, singleton};
use string::find_near;
use types::Type;

/// The type of the SQL query.
enum SqlQueryType {
    Aggregate,
    CreateTable,
    Delete,
    Drop,
    Insert,
    Select,
    Update,
}

/// The query data gathered during the analyze.
type QueryData = (FilterExpression, Vec<Join>, Limit, Vec<Order>, Vec<Assignment>, Vec<TypedField>, Vec<Aggregate>, SqlQueryType);

/// Analyze and transform the AST.
pub fn analyze(method_calls: MethodCalls, sql_tables: &SqlTables) -> SqlResult<Query> {
    // TODO: vérifier que la suite d’appels de méthode est valide (de même que l’ordre pour filter).
    let mut errors = vec![];

    let table_name = method_calls.name.clone();
    if !sql_tables.contains_key(&table_name) {
        unknown_table_error(&table_name, method_calls.position, sql_tables, &mut errors);
    }

    check_methods(&method_calls, &mut errors);

    let table = sql_tables.get(&table_name);
    let calls = &method_calls.calls;
    let mut delete_position = None;

    let (fields, filter_expression, joins, limit, order, assignments, typed_fields, aggregates, query_type) =
        match table {
            Some(table) => {
                let (filter_expression, joins, limit, order, assignments, typed_fields, aggregates, query_type) =
                    try!(process_methods(&calls, table, &table_name, &mut delete_position));
                let fields = get_query_fields(table, &table_name, &joins, sql_tables);
                (fields, filter_expression, joins, limit, order, assignments, typed_fields, aggregates, query_type)

            },
            None => (vec![], FilterExpression::NoFilters, vec![], Limit::NoLimit, vec![], vec![], vec![], vec![], SqlQueryType::Select),
        };

    let query = new_query(fields, filter_expression, joins, limit, order, assignments, typed_fields, aggregates, query_type, table_name);

    check_delete(&query, delete_position, &mut errors);

    res(query, errors)
}

/// Analyze the literal types in the `Query`.
pub fn analyze_types(query: Query) -> SqlResult<Query> {
    let mut errors = vec![];
    match query {
        Query::Aggregate { ref filter, ref table, .. } => {
            analyze_filter_types(filter, &table, &mut errors);
        },
        Query::CreateTable { .. } => (), // Nothing to analyze.
        Query::Delete { ref filter, ref table } => {
            analyze_filter_types(filter, &table, &mut errors);
        },
        Query::Drop { .. } => (), // Nothing to analyze.
        Query::Insert { ref assignments, ref table } => {
            analyze_assignments_types(assignments, &table, &mut errors);
        },
        Query::Select { ref filter, ref limit, ref table, .. } => {
            analyze_filter_types(filter, &table, &mut errors);
            analyze_limit_types(limit, &mut errors);
        },
        Query::Update { ref assignments, ref filter, ref table } => {
            analyze_filter_types(filter, &table, &mut errors);
            analyze_assignments_types(assignments, &table, &mut errors);
        },
    }
    res(query, errors)
}

/// Check that `Delete` `Query` contains a filter.
fn check_delete(query: &Query, delete_position: Option<Span>, errors: &mut Vec<Error>) {
    if let Query::Delete { ref filter, .. } = *query {
        if let FilterExpression::NoFilters = *filter {
            errors.push(Error::new_warning(
                "delete() without filters".to_owned(),
                delete_position.unwrap(), // There is always a delete position when the query is of type Delete.
            ));
        }
    }
}

/// Check if the `identifier` is a field in the struct `table_name`.
pub fn check_field(identifier: &str, position: Span, table_name: &str, table: &SqlFields, errors: &mut Vec<Error>) {
    if !table.contains_key(identifier) {
        errors.push(Error::new(
            format!("attempted access of field `{}` on type `{}`, but no field with that name was found", identifier, table_name),
            position
        ));
        let field_names = table.keys();
        propose_similar_name(identifier, field_names, position, errors);
    }
}

/// Check if the type of `identifier` matches the type of the `value` expression.
fn check_field_type(table_name: &str, rvalue: &RValue, value: &Expression, errors: &mut Vec<Error>) {
    let field_type = get_field_type_by_rvalue(table_name, rvalue);
    check_type(field_type, value, errors);
}

/// Check if the method `calls` exist.
fn check_methods(method_calls: &MethodCalls, errors: &mut Vec<Error>) {
    let methods = vec![
        "aggregate".to_owned(),
        "all".to_owned(),
        "create".to_owned(),
        "delete".to_owned(),
        "drop".to_owned(),
        "filter".to_owned(),
        "get".to_owned(),
        "insert".to_owned(),
        "join".to_owned(),
        "limit".to_owned(),
        "sort".to_owned(),
        "update".to_owned(),
    ];
    for method_call in &method_calls.calls {
        if !methods.contains(&method_call.name) {
            errors.push(Error::new(
                format!("no method named `{}` found in tql", method_call.name),
                method_call.position,
            ));
            propose_similar_name(&method_call.name, methods.iter(), method_call.position, errors);
        }
    }

    if method_calls.calls.is_empty() {
        let table_name = &method_calls.name;
        errors.push(Error::new_with_code(format!("`{}` is the name of a struct, but this expression uses it like a method name", table_name), method_calls.position, "E0423"));
        errors.push(Error::new_help(
            format!("did you mean to write `{}.method()`?", table_name),
            method_calls.position,
        ));
    }
}

/// Check that the specified method call did not received any arguments.
fn check_no_arguments(method_call: &MethodCall, errors: &mut Vec<Error>) {
    if !method_call.arguments.is_empty() {
        let length = method_call.arguments.len();
        let plural_verb =
            if length == 1 {
                " was"
            }
            else {
                "s were"
            };
        errors.push(Error::new_with_code(format!("this method takes 0 parameters but {} parameter{} supplied", length, plural_verb), method_call.position, "E0061"));
    }
}

/// Check if the `field_type` is compatible with the `expression`'s type.
pub fn check_type(field_type: &Type, expression: &Expression, errors: &mut Vec<Error>) {
    if field_type != expression {
        let literal_type = get_type(expression);
        mismatched_types(field_type, &literal_type, expression.span, errors);
    }
}

/// Check if the `field_type` is compatible with the `rvalue`'s type.
fn check_type_rvalue(expected_type: &Type, rvalue: &Spanned<RValue>, table_name: &str, errors: &mut Vec<Error>) {
    let field_type = get_field_type_by_rvalue(table_name, &rvalue.node);
    if *field_type != *expected_type {
        mismatched_types(expected_type, &field_type, rvalue.span, errors);
    }
}

/// Convert the `arguments` to the `Type`.
fn convert_arguments<F, Type>(arguments: &[P<Expr>], table_name: &str, table: &SqlFields, convert_argument: F) -> SqlResult<Vec<Type>>
        where F: Fn(&Expression, &str, &SqlFields) -> SqlResult<Type> {
    let mut items = vec![];
    let mut errors = vec![];

    for arg in arguments {
        try(convert_argument(arg, table_name, table), &mut errors, |item| {
            items.push(item);
        });
    }

    res(items, errors)
}

/// Get the type of the field if it exists from an `RValue`.
fn get_field_type_by_rvalue<'a>(table_name: &'a str, rvalue: &RValue) -> &'a Type {
    // NOTE: At this stage (type analysis), the field exists, hence unwrap().
    match *rvalue {
        RValue::Identifier(ref identifier) => {
            get_field_type(table_name, identifier).unwrap()
        },
        RValue::MethodCall(ast::MethodCall { ref method_name, ref object_name, .. }) => {
            let tables = singleton();
            let table = tables.get(table_name).unwrap();
            let methods = methods_singleton();
            let typ = table.get(object_name).unwrap();
            let typ =
                match typ.node {
                    Type::Nullable(_) => Cow::Owned(Type::Nullable(box Type::Generic)),
                    ref typ => Cow::Borrowed(typ),
                };
            let type_methods = methods.get(&typ).unwrap();
            let method = type_methods.get(method_name).unwrap();
            &method.return_type
        },
    }
}

/// Get the query field fully qualified names.
fn get_query_fields(table: &SqlFields, table_name: &str, joins: &[Join], sql_tables: &SqlTables) -> Vec<Identifier> {
    let mut fields = vec![];
    for (field, typ) in table {
        match typ.node {
            // TODO: faire attention aux conflits de nom.
            Type::Custom(ref foreign_table) => {
                let table_name = foreign_table;
                if let Some(foreign_table) = sql_tables.get(foreign_table) {
                    if has_joins(&joins, field) {
                        for (field, typ) in foreign_table {
                            match typ.node {
                                Type::Custom(_) | Type::UnsupportedType(_) => (), // NOTE: Do not add foreign key recursively.
                                _ => {
                                    fields.push(table_name.clone() + "." + &field);
                                },
                            }
                        }
                    }
                }
                // TODO: Check if the foreign table exists instead of doing this in the lint plugin
                // (it is needed here because the related fields need to be included in the query.)
            },
            Type::UnsupportedType(_) => (),
            _ => {
                fields.push(table_name.to_owned() + "." + &field);
            },
        }
    }
    fields
}

/// Get the string representation of an literal `Expression` type.
fn get_type(expression: &Expression) -> &str {
    match expression.node {
        ExprLit(ref literal) => {
            match literal.node {
                LitBool(_) => "bool",
                LitByte(_) => "u8",
                LitByteStr(_) => "Vec<u8>",
                LitChar(_) => "char",
                LitFloat(_, FloatTy::TyF32) => "f32",
                LitFloat(_, FloatTy::TyF64) => "f64",
                LitFloatUnsuffixed(_) => "floating-point variable",
                LitInt(_, int_type) =>
                    match int_type {
                        SignedIntLit(IntTy::TyIs, _) => "isize",
                        SignedIntLit(IntTy::TyI8, _) => "i8",
                        SignedIntLit(IntTy::TyI16, _) => "i16",
                        SignedIntLit(IntTy::TyI32, _) => "i32",
                        SignedIntLit(IntTy::TyI64, _) => "i64",
                        UnsignedIntLit(UintTy::TyUs) => "usize",
                        UnsignedIntLit(UintTy::TyU8) => "u8",
                        UnsignedIntLit(UintTy::TyU16) => "u16",
                        UnsignedIntLit(UintTy::TyU32) => "u32",
                        UnsignedIntLit(UintTy::TyU64) => "u64",
                        UnsuffixedIntLit(_) => "integral variable",
                    }
                ,
                LitStr(_, _) => "String",
            }
        }
        _ => panic!("expression needs to be a literal"),
    }
}

/// Check if there is a join in `joins` on a field named `name`.
pub fn has_joins(joins: &[Join], name: &str) -> bool {
    joins.iter()
        .map(|join| &join.left_field)
        .any(|field_name| field_name == name)
}

/// Add a mismatched types error to `errors`.
fn mismatched_types<S: Display, T: Display>(expected_type: S, actual_type: &T, position: Span, errors: &mut Vec<Error>) {
    errors.push(Error::new_with_code(
        format!("mismatched types:\n expected `{}`,\n    found `{}`", expected_type, actual_type),
        position,
        "E0308",
    ));
    errors.push(Error::new_note(
        "in this expansion of sql! (defined in tql)".to_owned(),
        position, // TODO: mettre la position de l’appel de macro sql!.
    ));
}

/// Create a new query from all the data gathered by the method calls.
fn new_query(fields: Vec<Identifier>, filter_expression: FilterExpression, joins: Vec<Join>, limit: Limit, order: Vec<Order>, assignments: Vec<Assignment>, typed_fields: Vec<TypedField>, aggregates: Vec<Aggregate>, query_type: SqlQueryType, table_name: String) -> Query {
    match query_type {
        SqlQueryType::Aggregate =>
            Query::Aggregate {
                aggregates: aggregates,
                filter: filter_expression,
                joins: joins,
                table: table_name,
            },
        SqlQueryType::CreateTable =>
            Query::CreateTable {
                fields: typed_fields,
                table: table_name,
            },
        SqlQueryType::Delete =>
            Query::Delete {
                filter: filter_expression,
                table: table_name,
            },
        SqlQueryType::Drop =>
            Query::Drop {
                table: table_name,
            },
        SqlQueryType::Insert =>
            Query::Insert {
                assignments: assignments,
                table: table_name,
            },
        SqlQueryType::Select =>
            Query::Select {
                fields: fields,
                filter: filter_expression,
                joins: joins,
                limit: limit,
                order: order,
                table: table_name,
            },
        SqlQueryType::Update =>
            Query::Update {
                assignments: assignments,
                filter: filter_expression,
                table: table_name,
            },
    }
}

/// Create an error about a table not having a primary key.
pub fn no_primary_key(table_name: &str, position: Span) -> Error {
    Error::new(format!("Table {} does not have a primary key", table_name), position)
}

/// Gather data about the query in the method `calls`.
fn process_methods(calls: &[MethodCall], table: &SqlFields, table_name: &str, delete_position: &mut Option<Span>) -> SqlResult<QueryData> {
    let mut errors = vec![];
    let mut assignments = vec![];
    let mut filter_expression = FilterExpression::NoFilters;
    let mut joins = vec![];
    let mut limit = Limit::NoLimit;
    let mut order = vec![];
    let mut query_type = SqlQueryType::Select;
    let mut typed_fields = vec![];
    let mut aggregates = vec![];
    for method_call in calls {
        match &method_call.name[..] {
            "aggregate" => {
                try(convert_arguments(&method_call.arguments, &table_name, table, argument_to_aggregate), &mut errors, |aggrs| {
                    aggregates = aggrs;
                });
                query_type = SqlQueryType::Aggregate;
            },
            "all" => {
                check_no_arguments(&method_call, &mut errors);
            },
            "create" => {
                check_no_arguments(&method_call, &mut errors);
                query_type = SqlQueryType::CreateTable;
                for (field, typ) in table {
                    typed_fields.push(TypedField {
                        identifier: field.clone(),
                        typ: typ.node.to_sql(),
                    });
                }
            },
            "delete" => {
                check_no_arguments(&method_call, &mut errors);
                query_type = SqlQueryType::Delete;
                *delete_position = Some(method_call.position);
            },
            "drop" => {
                check_no_arguments(&method_call, &mut errors);
                query_type = SqlQueryType::Drop;
            },
            "filter" => {
                try(expression_to_filter_expression(&method_call.arguments[0], &table_name, table), &mut errors, |filter| {
                    filter_expression = filter;
                });
            },
            "get" => {
                // TODO: la méthode get() accepte d’être utilisée sans argument.
                try(get_expression_to_filter_expression(&method_call.arguments[0], &table_name, table), &mut errors, |(filter, new_limit)| {
                    filter_expression = filter;
                    limit = new_limit;
                });
            },
            "insert" => {
                try(convert_arguments(&method_call.arguments, &table_name, table, argument_to_assignment), &mut errors, |assigns| {
                    assignments = assigns;
                });
                check_insert_arguments(&assignments, method_call.position, &table, &mut errors);
                query_type = SqlQueryType::Insert;
            },
            "join" => {
                try(convert_arguments(&method_call.arguments, &table_name, table, argument_to_join), &mut errors, |mut new_joins| {
                    joins.append(&mut new_joins);
                });
            },
            "limit" => {
                try(arguments_to_limit(&method_call.arguments[0]), &mut errors, |new_limit| {
                    limit = new_limit;
                });
            },
            "sort" => {
                try(convert_arguments(&method_call.arguments, &table_name, table, argument_to_order), &mut errors, |new_order| {
                    order = new_order;
                });
            },
            "update" => {
                try(convert_arguments(&method_call.arguments, &table_name, table, argument_to_assignment), &mut errors, |assigns| {
                    assignments = assigns;
                });
                query_type = SqlQueryType::Update;
            },
            _ => (), // NOTE: Nothing to do since check_methods() check for unknown method.
        }
    }
    res((filter_expression, joins, limit, order, assignments, typed_fields, aggregates, query_type), errors)
}

/// Check if a name similar to `identifier` exists in `choices` and show a message if one exists.
/// Returns true if a similar name was found.
pub fn propose_similar_name<'a, T: Iterator<Item = &'a String>>(identifier: &str, choices: T, position: Span, errors: &mut Vec<Error>) -> bool {
    if let Some(name) = find_near(&identifier, choices) {
        errors.push(Error::new_help(
            format!("did you mean {}?", name),
            position,
        ));
        true
    }
    else {
        false
    }
}

/// If `result` is an `Err`, add the errors to `errors`.
/// Otherwise, execute the closure.
fn try<F: FnMut(T), T>(mut result: Result<T, Vec<Error>>, errors: &mut Vec<Error>, mut fn_using_result: F) {
    match result {
        Ok(value) => fn_using_result(value),
        Err(ref mut errs) => errors.append(errs),
    }
}

/// Add an error to the vector `errors` about an unknown SQL table.
/// It suggests a similar name if there exist one.
pub fn unknown_table_error(table_name: &str, position: Span, sql_tables: &SqlTables, errors: &mut Vec<Error>) {
    errors.push(Error::new_with_code(
        format!("`{}` does not name an SQL table", table_name),
        position,
        "E0422",
    ));
    let tables = sql_tables.keys();
    if !propose_similar_name(&table_name, tables, position, errors) {
        errors.push(Error::new_help(
            format!("did you forget to add the #[sql_table] attribute on the {} struct?", table_name),
            position,
        ));
    }
}
