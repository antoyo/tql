//! Semantic analyzer.

use syntax::ast::{BinOp_, Expr, Path};
use syntax::ast::Expr_::{ExprBinary, ExprCall, ExprCast, ExprLit, ExprMethodCall, ExprParen, ExprPath, ExprRange, ExprUnary};
use syntax::ast::FloatTy;
use syntax::ast::IntTy;
use syntax::ast::Lit_::{LitBool, LitByte, LitByteStr, LitChar, LitFloat, LitFloatUnsuffixed, LitInt, LitStr};
use syntax::ast::LitIntType::{SignedIntLit, UnsignedIntLit, UnsuffixedIntLit};
use syntax::ast::UintTy;
use syntax::ast::UnOp::{UnNeg, UnNot};
use syntax::codemap::{DUMMY_SP, Span, Spanned};
use syntax::ptr::P;

use ast::{Expression, Filter, FilterExpression, Filters, Identifier, Join, Limit, LogicalOperator, Order, RelationalOperator, Query};
use ast::Limit::{EndRange, Index, LimitOffset, NoLimit, Range, StartRange};
use error::{Error, SqlResult, res};
use parser::{MethodCall, MethodCalls};
use plugin::number_literal;
use state::{SqlFields, SqlTables, Type, singleton};
use string::find_near;

/// Analyze and transform the AST.
pub fn analyze<'a, 'b>(method_calls: MethodCalls, sql_tables: &'a SqlTables) -> SqlResult<Query<'b>> {
    // TODO: vérifier que la suite d’appels de méthode est valide (de même que l’ordre pour filter).
    let mut errors = vec![];

    let table_name = method_calls.name;
    let table = sql_tables.get(&table_name);
    let calls = &method_calls.calls;

    if !sql_tables.contains_key(&table_name) {
        unknown_table_error(&table_name, method_calls.position, sql_tables, &mut errors);
    }

    check_methods(&calls, &mut errors);

    let (fields, filter_expression, joins, limit, order) =
        match table {
            Some(table) => {
                let (filter_expression, joins, limit, order) = try!(process_methods(&calls, table, &table_name));
                let fields = get_query_fields(table, &table_name, &joins, sql_tables, &mut errors);
                (fields, filter_expression, joins, limit, order)

            },
            None => (vec![], FilterExpression::NoFilters, vec![], Limit::NoLimit, vec![]),
        };

    res(Query::Select {
        fields: fields,
        filter: filter_expression,
        joins: joins,
        limit: limit,
        order: order,
        table: table_name,
    }, errors)
}

fn analyze_filter_types(filter: &FilterExpression, table_name: &str, errors: &mut Vec<Error>) {
    // TODO: vérifier que les opérateurs sont utilisé avec les bons types.
    match *filter {
        FilterExpression::Filter(ref filter) => {
            let tables = singleton();
            // TODO: ne pas utiliser unwrap().
            let table = tables.get(table_name).unwrap();
            let field_type = table.get(&filter.operand1).unwrap();
            check_type(field_type, &filter.operand2, errors);
        },
        FilterExpression::Filters(ref filters) => {
            analyze_filter_types(&*filters.operand1, table_name, errors);
            analyze_filter_types(&*filters.operand2, table_name, errors);
        },
        FilterExpression::NegFilter(ref filter) => {
            analyze_filter_types(filter, table_name, errors);
        },
        FilterExpression::NoFilters => (),
        FilterExpression::ParenFilter(ref filter) => {
            analyze_filter_types(filter, table_name, errors);
        }
    }
}

fn analyze_limit_types(limit: &Limit, errors: &mut Vec<Error>) {
    match *limit {
        EndRange(ref expression) => check_type(&Type::I64, expression, errors),
        Index(ref expression) => check_type(&Type::I64, expression, errors),
        LimitOffset(ref expression1, ref expression2) => {
            check_type(&Type::I64, expression1, errors);
            check_type(&Type::I64, expression2, errors);
        },
        NoLimit => (),
        Range(ref expression1, ref expression2) => {
            check_type(&Type::I64, expression1, errors);
            check_type(&Type::I64, expression2, errors);
        },
        StartRange(ref expression) => check_type(&Type::I64, expression, errors),
    }
}

/// Analyze the literal types in the `Query`.
pub fn analyze_types(query: Query) -> SqlResult<Query> {
    let mut errors = vec![];
    match query {
        Query::CreateTable { .. } => (), // TODO
        Query::Delete { .. } => (), // TODO
        Query::Insert { .. } => (), // TODO
        Query::Select { ref filter, ref limit, ref table, .. } => {
            analyze_filter_types(filter, &table, &mut errors);
            analyze_limit_types(limit, &mut errors);
        },
        Query::Update { .. } => (), // TODO
    }
    res(query, errors)
}

fn argument_to_join<'a>(arg: &Expr, table_name: &str, table: &SqlFields) -> SqlResult<'a, Join> {
    let mut errors = vec![];
    let mut join = Join {
        left_field: "".to_owned(),
        left_table: "".to_owned(),
        right_field: "".to_owned(),
        right_table: "".to_owned(),
    };

    match arg.node {
        ExprPath(None, ref path) => {
            let identifier = path.segments[0].identifier.to_string();
            check_field(&identifier, path.span, table_name, table, &mut errors);
            match table.get(&identifier) {
                Some(related_table_name) => {
                    join = Join {
                        left_field: identifier,
                        left_table: table_name.to_owned(),
                        right_field: "id".to_owned(),
                        right_table: related_table_name.to_string(),
                    };
                },
                None => (), // This case is handled the check_field() call above.
            }
        }
        _ => {
            errors.push(Error::new(
                "Expected identifier".to_owned(),
                arg.span,
            ));
        }
    }
    res(join, errors)
}

fn argument_to_order<'a>(arg: &Expr, table_name: &str, table: &SqlFields) -> SqlResult<'a, Order> {
    fn identifier<'a>(arg: &Expr, identifier: &Expr, table_name: &str, table: &SqlFields) -> SqlResult<'a, String> {
        let mut errors = vec![];
        if let ExprPath(_, Path { ref segments, span, .. }) = identifier.node {
            if segments.len() == 1 {
                let identifier = segments[0].identifier.to_string();
                check_field(&identifier, span, table_name, table, &mut errors);
                return res(identifier, errors);
            }
        }
        Err(vec![Error::new(
            "Expected an identifier".to_owned(),
            arg.span,
        )])
    }

    let mut errors = vec![];
    let order =
        match arg.node {
            ExprUnary(UnNeg, ref expr) => {
                let ident = try!(identifier(arg, expr, table_name, table));
                Order::Descending(ident)
            }
            ExprPath(None, ref path) => {
                let identifier = path.segments[0].identifier.to_string();
                check_field(&identifier, path.span, table_name, table, &mut errors);
                Order::Ascending(identifier)
            }
            _ => {
                errors.push(Error::new(
                    "Expected - or identifier".to_owned(),
                    arg.span,
                ));
                Order::Ascending("".to_owned())
            }
        };
    res(order, errors)
}

fn arguments_to_joints<'a>(arguments: &[P<Expr>], table_name: &str, table: &SqlFields) -> SqlResult<'a, Vec<Join>> {
    let mut joins = vec![];
    let mut errors = vec![];

    for arg in arguments {
        try(argument_to_join(arg, table_name, table), &mut errors, |join| {
            joins.push(join);
        });
    }

    res(joins, errors)
}

fn arguments_to_limit<'a, 'b>(expression: &'b P<Expr>) -> SqlResult<'a, Limit> {
    let mut errors = vec![];
    let limit =
        match expression.node {
            ExprRange(None, Some(ref range_end)) => {
                Limit::EndRange(range_end.clone())
            }
            ExprRange(Some(ref range_start), None) => {
                Limit::StartRange(range_start.clone())
            }
            ExprRange(Some(ref range_start), Some(ref range_end)) => {
                // TODO: vérifier que range_start < range_end.
                Limit::Range(range_start.clone(), range_end.clone())
            }
            ExprLit(_) | ExprPath(_, _) | ExprCall(_, _) | ExprMethodCall(_, _, _) | ExprBinary(_, _, _) | ExprUnary(_, _) | ExprCast(_, _)  => {
                Limit::Index(expression.clone())
            }
            _ => {
                errors.push(Error::new(
                    "Expected index range or number expression".to_owned(),
                    expression.span,
                ));
                Limit::NoLimit
            }
        };

    // TODO: vérifier si la limite ou le décalage est 0. Le cas échéant, ne pas les mettre dans
    // la requête (optimisation).

    res(limit, errors)
}

fn arguments_to_orders<'a>(arguments: &[P<Expr>], table_name: &str, table: &SqlFields) -> SqlResult<'a, Vec<Order>> {
    let mut orders = vec![];
    let mut errors = vec![];

    for arg in arguments {
        try(argument_to_order(arg, table_name, table), &mut errors, |order| {
            orders.push(order);
        });
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

fn check_field(identifier: &str, position: Span, table_name: &str, table: &SqlFields, errors: &mut Vec<Error>) {
    if !table.contains_key(identifier) {
        errors.push(Error::new(
            format!("attempted access of field `{}` on type `{}`, but no field with that name was found", identifier, table_name),
            position
        ));
        let field_names = table.keys();
        if let Some(name) = find_near(identifier, field_names) {
            errors.push(Error::new_help(
                format!("did you mean {}?", name),
                position
            ));
        }
    }
}

fn check_methods(calls: &[MethodCall], errors: &mut Vec<Error>) {
    let methods = vec![
        "all".to_owned(),
        "filter".to_owned(),
        "get".to_owned(),
        "join".to_owned(),
        "limit".to_owned(),
        "sort".to_owned(),
    ];
    for method_call in calls {
        if !methods.contains(&method_call.name) {
            errors.push(Error::new(
                format!("no method named `{}` found in tql", method_call.name),
                method_call.position,
            ));
            if let Some(name) = find_near(&method_call.name, methods.iter()) {
                errors.push(Error::new_help(
                    format!("did you mean {}?", name),
                    method_call.position,
                ));
            }
        }
    }
}

fn check_type(field_type: &Type, expression: &Expression, errors: &mut Vec<Error>) {
    if !same_type(field_type, expression) {
        let literal_type = get_type(expression);
        errors.push(Error::new_with_code(
            format!("mismatched types:\n expected `{}`,\n    found `{}`", field_type, literal_type),
            expression.span,
            "E0308",
        ));
        errors.push(Error::new_note(
            "in this expansion of sql! (defined in tql)".to_owned(),
            expression.span, // TODO: mettre la position de l’appel de macro sql!.
        ));
    }
}

/// Convert a Rust expression to a `FilterExpression`.
fn expression_to_filter_expression<'a>(arg: &P<Expr>, table_name: &str, table: &SqlFields) -> SqlResult<'a, FilterExpression> {
    let mut errors = vec![];

    let dummy = FilterExpression::NoFilters;
    let filter =
        match arg.node {
            ExprBinary(Spanned { node: op, .. }, ref expr1, ref expr2) => {
                match expr1.node {
                    ExprPath(None, ref path) => {
                        let identifier = path.segments[0].identifier.to_string();
                        check_field(&identifier, path.span, table_name, table, &mut errors);
                        FilterExpression::Filter(Filter {
                            operand1: identifier,
                            operator: binop_to_relational_operator(op),
                            operand2: expr2.clone(),
                        })
                    },
                    ExprBinary(_, _, _) | ExprUnary(UnNot, _) | ExprParen(_) => {
                        // TODO: accumuler les erreurs au lieu d’arrêter à la première.
                        let filter1 = try!(expression_to_filter_expression(expr1, table_name, table));
                        let filter2 = try!(expression_to_filter_expression(expr2, table_name, table));
                        FilterExpression::Filters(Filters {
                            operand1: Box::new(filter1),
                            operator: binop_to_logical_operator(op),
                            operand2: Box::new(filter2),
                        })
                    },
                    _ => {
                        errors.push(Error::new(
                            "Expected identifier or binary operation".to_owned(),
                            expr1.span,
                        ));
                        dummy
                    },
                }
            },
            ExprUnary(UnNot, ref expr) => {
                let filter = try!(expression_to_filter_expression(expr, table_name, table));
                FilterExpression::NegFilter(box filter)
            },
            ExprParen(ref expr) => {
                let filter = try!(expression_to_filter_expression(expr, table_name, table));
                FilterExpression::ParenFilter(box filter)
            },
            _ => {
                errors.push(Error::new(
                    "Expected binary operation".to_owned(),
                    arg.span,
                ));
                dummy
            },
        };

    res(filter, errors)
}

fn get_expression_to_filter_expression<'a>(arg: &P<Expr>, table_name: &str, table: &SqlFields) -> SqlResult<'a, (FilterExpression, Limit)> {
    match arg.node {
        ExprLit(_) | ExprPath(_, _) => {
            let filter = FilterExpression::Filter(Filter {
                operand1: "id".to_owned(),
                operator: RelationalOperator::Equal,
                operand2: arg.clone(),
            });
            res((filter, Limit::NoLimit), vec![])
        },
        _ => {
            match expression_to_filter_expression(arg, table_name, table) {
                Ok(filter) => res((filter, Limit::Index(number_literal(0))), vec![]),
                Err(errors) => res((FilterExpression::NoFilters, Limit::NoLimit), errors),
            }
        },
    }
}

fn get_query_fields(table: &SqlFields, table_name: &str, joins: &[Join], sql_tables: &SqlTables, errors: &mut Vec<Error>) -> Vec<Identifier> {
    let mut fields = vec![];
    for (field, typ) in table {
        match *typ {
            // TODO: faire attention aux conflits de nom.
            Type::Custom(ref foreign_table) => {
                let table_name = foreign_table;
                match sql_tables.get(foreign_table) {
                    Some(foreign_table) => {
                        if has_joins(&joins, field) {
                            for (field, typ) in foreign_table {
                                match *typ {
                                    Type::Custom(_) | Type::Dummy => (), // Do not add foreign key recursively.
                                    _ => {
                                        fields.push(table_name.clone() + "." + &field);
                                    },
                                }
                            }
                        }
                    },
                    None => {
                        // TODO: utiliser la vraie position.
                        unknown_table_error(foreign_table, DUMMY_SP, &sql_tables, errors);
                    },
                }
            },
            Type::Dummy => (),
            _ => {
                fields.push(table_name.to_owned() + "." + &field);
            },
        }
    }
    fields
}

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
        _ => "",
    }
}

pub fn has_joins(joins: &[Join], name: &str) -> bool {
    joins.iter()
        .map(|join| &join.left_field)
        .any(|field_name| field_name == name)
}

fn process_methods<'a>(calls: &[MethodCall], table: &SqlFields, table_name: &str) -> SqlResult<'a, (FilterExpression, Vec<Join>, Limit, Vec<Order>)> {
    let mut errors = vec![];
    let mut filter_expression = FilterExpression::NoFilters;
    let mut joins = vec![];
    let mut limit = Limit::NoLimit;
    let mut order = vec![];
    for method_call in calls {
        match &method_call.name[..] {
            "all" => (), // TODO
            "filter" => {
                try(expression_to_filter_expression(&method_call.arguments[0], &table_name, table), &mut errors, |filter| {
                    filter_expression = filter;
                });
            },
            "get" => {
                try(get_expression_to_filter_expression(&method_call.arguments[0], &table_name, table), &mut errors, |(filter, new_limit)| {
                    filter_expression = filter;
                    limit = new_limit;
                });
            },
            "join" => {
                try(arguments_to_joints(&method_call.arguments, &table_name, table), &mut errors, |mut new_joins| {
                    joins.append(&mut new_joins);
                });
            },
            "limit" => {
                try(arguments_to_limit(&method_call.arguments[0]), &mut errors, |new_limit| {
                    limit = new_limit;
                });
            },
            "sort" => {
                try(arguments_to_orders(&method_call.arguments, &table_name, table), &mut errors, |new_order| {
                    order = new_order;
                });
            },
            _ => (), // Nothing to do since check_methods() check for unknown method.
        }
    }
    res((filter_expression, joins, limit, order), errors)
}

fn same_type(field_type: &Type, expression: &Expression) -> bool {
    match expression.node {
        ExprLit(ref literal) => {
            match literal.node {
                LitBool(_) => *field_type == Type::Bool,
                LitByte(_) => false,
                LitByteStr(_) => *field_type == Type::ByteString,
                LitChar(_) => *field_type == Type::Char,
                LitFloat(_, FloatTy::TyF32) => *field_type == Type::F32,
                LitFloat(_, FloatTy::TyF64) => *field_type == Type::F64,
                LitFloatUnsuffixed(_) => *field_type == Type::F32 || *field_type == Type::F64,
                LitInt(_, int_type) =>
                    match int_type {
                        SignedIntLit(IntTy::TyIs, _) => false,
                        SignedIntLit(IntTy::TyI8, _) => *field_type == Type::I8,
                        SignedIntLit(IntTy::TyI16, _) => *field_type == Type::I16,
                        SignedIntLit(IntTy::TyI32, _) => *field_type == Type::I32 || *field_type == Type::Serial,
                        SignedIntLit(IntTy::TyI64, _) => *field_type == Type::I64,
                        UnsignedIntLit(UintTy::TyU32) => *field_type == Type::U32,
                        UnsignedIntLit(_) => false,
                        UnsuffixedIntLit(_) => *field_type == Type::I32 || *field_type == Type::U32 || *field_type == Type::Serial,
                    }
                ,
                LitStr(_, _) => *field_type == Type::String,
            }
        }
        _ => true, // Returns true, because the type checking for non-literal is done later.
    }
}

/// If `result` is an `Err`, add the errors to `errors`.
/// Otherwise, execute the closure.
fn try<'a, F: FnMut(T), T>(mut result: Result<T, Vec<Error<'a>>>, errors: &mut Vec<Error<'a>>, mut fn_using_result: F) {
    match result {
        Ok(value) => fn_using_result(value),
        Err(ref mut errs) => errors.append(errs),
    }
}

fn unknown_table_error(table_name: &str, position: Span, sql_tables: &SqlTables, errors: &mut Vec<Error>) {
    errors.push(Error::new_with_code(
        format!("`{}` does not name an SQL table", table_name),
        position,
        "E0422",
    ));
    let tables = sql_tables.keys();
    if let Some(name) = find_near(&table_name, tables) {
        errors.push(Error::new_help(
            format!("did you mean {}?", name),
            position,
        ));
    }
    else {
        errors.push(Error::new_help(
            format!("did you forget to add the #[sql_table] attribute on the {} struct?", table_name),
            position,
        ));
    }
}
