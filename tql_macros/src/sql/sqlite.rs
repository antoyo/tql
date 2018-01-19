/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! The SQLite code generator.

use std::iter;
use std::str::from_utf8;

use proc_macro2::Span;
use quote::Tokens;
use syn::{Expr, Ident, Lit};

use ast::{
    Aggregate,
    AggregateFilter,
    AggregateFilterExpression,
    AggregateFilters,
    Assignment,
    AssignementOperator,
    Expression,
    Filter,
    Filters,
    FilterExpression,
    FilterValue,
    Identifier,
    Join,
    Limit,
    LogicalOperator,
    MethodCall,
    Order,
    RelationalOperator,
    Query,
    TypedField,
};
use ast::Limit::{
    EndRange,
    Index,
    LimitOffset,
    NoLimit,
    Range,
    StartRange,
};
use plugin::string_literal;
use state::methods_singleton;
use types::Type;

/// A generic trait for converting a value to SQL.
pub trait ToSql {
    fn to_sql(&self, index: &mut usize) -> String;
}

impl ToSql for Aggregate {
    fn to_sql(&self, index: &mut usize) -> String {
        // TODO: do not hard-code the type.
        self.function.to_sql(index) + "(" + &self.field.expect("Aggregate field").to_sql(index) + ")"
    }
}

impl ToSql for [Aggregate] {
    fn to_sql(&self, index: &mut usize) -> String {
        self.iter().map(|aggregate| aggregate.to_sql(index)).collect::<Vec<_>>().join(", ")
    }
}

impl ToSql for AggregateFilter {
    fn to_sql(&self, index: &mut usize) -> String {
        self.operand1.to_sql(index) + " " +
            &self.operator.to_sql(index) + " " +
            &self.operand2.to_sql(index)
    }
}

impl ToSql for AggregateFilterExpression {
    fn to_sql(&self, index: &mut usize) -> String {
        match *self {
            AggregateFilterExpression::Filter(ref filter) => filter.to_sql(index),
            AggregateFilterExpression::Filters(ref filters) => filters.to_sql(index),
            AggregateFilterExpression::NegFilter(ref filter) =>
                "NOT ".to_string() +
                &filter.to_sql(index),
            AggregateFilterExpression::NoFilters => "".to_string(),
            AggregateFilterExpression::ParenFilter(ref filter) =>
                "(".to_string() +
                &filter.to_sql(index) +
                ")",
            AggregateFilterExpression::FilterValue(ref filter_value) => filter_value.node.to_sql(index),
        }
    }
}

impl ToSql for AggregateFilters {
    fn to_sql(&self, index: &mut usize) -> String {
        self.operand1.to_sql(index) + " " +
            &self.operator.to_sql(index) + " " +
            &self.operand2.to_sql(index)
    }
}

impl ToSql for Assignment {
    fn to_sql(&self, index: &mut usize) -> String {
        let identifier = self.identifier.expect("Assignment identifier").to_sql(index);
        if let AssignementOperator::Equal = self.operator.node {
            identifier + &self.operator.node.to_sql(index) + &self.value.to_sql(index)
        }
        else {
            identifier.clone() +
                &self.operator.node.to_sql(index).replace("{}", &identifier) +
                &self.value.to_sql(index)
        }
    }
}

impl ToSql for [Assignment] {
    fn to_sql(&self, index: &mut usize) -> String {
        self.iter().map(|assignment| assignment.to_sql(index)).collect::<Vec<_>>().join(", ")
    }
}

impl ToSql for AssignementOperator {
    fn to_sql(&self, _index: &mut usize) -> String {
        match *self {
            AssignementOperator::Add => " = {} + ",
            AssignementOperator::Divide => " = {} / ",
            AssignementOperator::Equal => " = ",
            AssignementOperator::Modulo => " = {} % ",
            AssignementOperator::Mul => " = {} * ",
            AssignementOperator::Sub => " = {} - ",
        }.to_string()
    }
}

/// Convert a literal expression to its SQL representation.
/// A non-literal is converted to ? for use with query parameters.
impl ToSql for Expression {
    fn to_sql(&self, index: &mut usize) -> String {
        match *self {
            Expr::Lit(ref literal) => {
                match literal.lit {
                    Lit::Bool(ref boolean) => boolean.value.to_string().to_uppercase(),
                    Lit::Byte(ref byte) =>
                        "'".to_string() +
                        &escape((byte.value() as char).to_string()) +
                        "'",
                        Lit::ByteStr(ref bytestring) =>
                            "'".to_string() +
                            // TODO: check if using unwrap() is secure here.
                            &escape(from_utf8(&bytestring.value()).unwrap().to_string()) +
                            "'",
                            Lit::Char(ref character) =>
                                "'".to_string() +
                                &escape(character.value().to_string()) +
                                "'",
                                Lit::Float(ref float) => float.value().to_string(),
                                Lit::Int(ref int) => int.value().to_string(),
                                Lit::Str(ref string) =>
                                    "'".to_string() +
                                    &escape(string.value()) +
                                    "'",
                                Lit::Verbatim(_) => panic!("Unsupported integer bigger than 64-bits"),
                }
            },
            _ => {
                let sql = format!("${}", index);
                *index += 1;
                sql
            },
        }
    }
}

impl ToSql for [Expression] {
    fn to_sql(&self, index: &mut usize) -> String {
        self.iter().map(|expr| expr.to_sql(index)).collect::<Vec<_>>().join(", ")
    }
}

impl ToSql for Vec<String> {
    fn to_sql(&self, _index: &mut usize) -> String {
        self.join(", ")
    }
}

impl Filter {
    fn to_tokens(&self, index: &mut usize) -> Tokens {
        let operand1 = self.operand1.to_tokens();
        let operator = self.operator.to_sql(index);
        let operand2 = self.operand2.to_sql(index);
        quote! {
            #operand1, " ", #operator, " ", #operand2
        }
    }
}

impl FilterExpression {
    fn to_tokens(&self, index: &mut usize) -> Tokens {
        match *self {
            FilterExpression::Filter(ref filter) => filter.to_tokens(index),
            FilterExpression::Filters(ref filters) => filters.to_tokens(index),
            FilterExpression::NegFilter(ref filter) => {
                let filter = filter.to_tokens(index);
                quote! {
                    "NOT ", #filter
                }
            },
            FilterExpression::NoFilters => quote! { "" }, // No filters result in no SQL code.
            FilterExpression::ParenFilter(ref filter) => {
                let filter = filter.to_tokens(index);
                quote! {
                    "(", #filter, ")"
                }
            }
            FilterExpression::FilterValue(ref filter_value) => filter_value.node.to_tokens(),
        }
    }
}

impl Filters {
    fn to_tokens(&self, index: &mut usize) -> Tokens {
        let operand1 = self.operand1.to_tokens(index);
        let operator = self.operator.to_sql(index);
        let operand2 = self.operand2.to_tokens(index);
        quote! {
            #operand1, " ", #operator, " ", #operand2
        }
    }
}

impl ToSql for Ident {
    fn to_sql(&self, _index: &mut usize) -> String {
        self.to_string()
    }
}

impl ToSql for [Ident] {
    fn to_sql(&self, index: &mut usize) -> String {
        self.iter().map(|ident| ident.to_sql(index)).collect::<Vec<_>>().join(", ")
    }
}

impl FilterValue {
    fn to_tokens(&self) -> Tokens {
        let sql =
            match *self {
                FilterValue::Identifier(ref table, ref identifier) => format!("{}.{}", table, identifier.to_sql(&mut 1)),
                FilterValue::MethodCall(MethodCall { ref arguments, ref object_name, ref method_name, ..  }) => {
                    let methods = methods_singleton();
                    if let Some(method) = methods.get(&method_name.to_string()) {
                        // In the template, $0 represents the object identifier and $1, $2, ... the
                        // arguments.
                        let mut sql = method.template.replace("$0", &object_name.to_string());
                        let mut index = 1;
                        for argument in arguments {
                            sql = sql.replace(&format!("${}", index), &argument.to_sql(&mut 1));
                            index += 1;
                        }
                        sql
                    }
                    else {
                        // NOTE: type checking will disallow this code to be executed.
                        String::new()
                    }
                },
                FilterValue::None => unreachable!("FilterValue::None in FilterValue::to_sql()"),
                FilterValue::PrimaryKey(ref table) => {
                    let macro_name = Ident::new(&format!("tql_{}_primary_key_field", table), Span::call_site());
                    return quote! {
                        #table, ".", #macro_name!()
                    };
                },
            };
        let expr = string_literal(&sql);
        quote! {
            #expr
        }
    }
}

impl Join {
    fn to_check(&self) -> Tokens {
        let related_table_macro_name =
            Ident::new(&format!("tql_{}_check_related_tables", self.base_table), self.base_field.span);
        let related_pk_macro_name = Ident::new(&format!("tql_{}_check_related_pks", self.base_table),
            self.base_field.span);
        let base_field_ident = &self.base_field;
        quote_spanned! { self.base_field.span =>
            #related_table_macro_name!(#base_field_ident);
            #related_pk_macro_name!(#base_field_ident);
        }
    }

    fn to_tokens(&self) -> Tokens {
        let related_table_macro_name =
            Ident::new(&format!("tql_{}_related_tables", self.base_table), Span::call_site());
        let related_pks_macro_name = Ident::new(&format!("tql_{}_related_pks", self.base_table), self.base_field.span);
        let base_table = &self.base_table;
        let base_field = self.base_field.to_sql(&mut 1);
        let base_field_ident = &self.base_field;
        let related_table_name = quote_spanned! { Span::call_site() =>
            #related_table_macro_name!(#base_field_ident)
        };
        quote! {
            " INNER JOIN ", #related_table_name, " ON ", #base_table, ".", #base_field, " = ",
                    #related_table_name, ".", #related_pks_macro_name!(#base_field_ident)
        }
    }
}

fn sep_by<I: Iterator<Item=Tokens>>(elements: I, sep: &str) -> Tokens {
    let mut elements: Vec<_> = elements.collect();
    if let Some(last_element) = elements.pop() {
        let elements = elements.iter()
            .map(|element|
                 quote! {
                     #element, #sep
                 }
                );
        quote! {
            #(#elements,)* #last_element
        }
    }
    else {
        quote! {
            ""
        }
    }
}

fn joins_to_check(joins: &[Join]) -> Tokens {
    let checks = joins.iter().map(|join| join.to_check());
    quote! {
        #(#checks)*
    }
}

fn joins_to_tokens(joins: &[Join]) -> Tokens {
    sep_by(joins.iter().map(|join| join.to_tokens()), " ")
}

fn joined_fields(joins: &[Join], table: &str) -> Tokens {
    let macro_name = Ident::new(&format!("tql_{}_related_field_list", table), Span::call_site());
    let fields = joins.iter()
        .map(|join| join.base_field);
    let macro_name = iter::repeat(macro_name)
        .take(joins.len());
    quote! {
        #(, ", ", #macro_name!(#fields))*
    }
}

impl ToSql for Identifier {
    fn to_sql(&self, _index: &mut usize) -> String {
        self.clone()
    }
}

impl ToSql for Limit {
    fn to_sql(&self, index: &mut usize) -> String {
        match *self {
            EndRange(ref expression) => " LIMIT ".to_string() + &expression.to_sql(index),
            Index(ref expression) =>
                " LIMIT 1".to_string() +
                    " OFFSET " + &expression.to_sql(index),
            LimitOffset(ref expression1, ref expression2) =>
                " LIMIT ".to_string() + &expression1.to_sql(index) +
                    " OFFSET " + &expression2.to_sql(index),
            NoLimit => "".to_string(),
            Range(ref expression1, ref expression2) =>
                " LIMIT ".to_string() + &expression2.to_sql(index) +
                    " OFFSET " + &expression1.to_sql(index),
            StartRange(ref expression) => " LIMIT -1 OFFSET ".to_string() + &expression.to_sql(index),
        }
    }
}

impl ToSql for LogicalOperator {
    fn to_sql(&self, _index: &mut usize) -> String {
        match *self {
            LogicalOperator::And => "AND",
            LogicalOperator::Not => "NOT",
            LogicalOperator::Or => "OR",
        }.to_string()
    }
}

impl ToSql for Order {
    fn to_sql(&self, index: &mut usize) -> String {
        match *self {
            Order::Ascending(ref field) => field.to_sql(index),
            Order::Descending(ref field) => field.to_sql(index) + " DESC",
            Order::NoOrder => String::new(),
        }
    }
}

impl ToSql for [Order] {
    fn to_sql(&self, index: &mut usize) -> String {
        self.iter().map(|order| order.to_sql(index)).collect::<Vec<_>>().join(", ")
    }
}

/// Convert a whole `Query` to SQL.
impl Query {
    pub fn to_tokens(&self) -> Tokens {
        match *self {
            Query::Aggregate { ref aggregates, ref aggregate_filter, ref filter, ref groups, ref joins, ref table } => {
                let where_clause = filter_to_where_clause(filter);
                let group_clause =
                    if !groups.is_empty() {
                        " GROUP BY "
                    }
                    else {
                        ""
                    };
                let having_clause =
                    if let AggregateFilterExpression::NoFilters = *aggregate_filter {
                        ""
                    }
                    else {
                        " HAVING "
                    };
                let aggregates = aggregates.to_sql(&mut 1);
                let check_joins = joins_to_check(&joins);
                let joins = joins_to_tokens(&joins);
                let index = &mut 1;
                let filter = filter.to_tokens(index);
                let groups = groups.to_sql(&mut 1);
                let aggregate_filter = aggregate_filter.to_sql(index);
                quote! {{
                    #check_joins
                    concat!("SELECT ", #aggregates, " FROM ", #table, #joins, #where_clause, #filter, #group_clause,
                            #groups, #having_clause, #aggregate_filter)
                }}
            },
            Query::CreateTable { ref table } => {
                let macro_name = Ident::new(&format!("tql_{}_create_query", table), Span::call_site());
                quote_spanned! { Span::call_site() =>
                    #macro_name!()
                }
            },
            Query::Delete { ref filter, ref table, use_pk: _use_pk } => {
                let where_clause = filter_to_where_clause(filter);
                let filter = filter.to_tokens(&mut 1);
                quote! {
                    concat!("DELETE FROM ", #table, #where_clause, #filter)
                }
            },
            Query::Drop { ref table } => {
                string_token(format!("DROP TABLE {table}", table = table).as_str())
            },
            Query::Insert { ref assignments, ref table } => {
                let fields: Vec<_> = assignments.iter().map(|assign|
                    assign.identifier.expect("Assignment identifier").to_sql(&mut 1)).collect();
                let index = &mut 1;
                let values: Vec<_> = assignments.iter().map(|assign|
                    assign.value.to_sql(index)
                ).collect();
                // Add the SQL code to get the inserted primary key.
                // TODO: what to do when there is no primary key?
                let query =
                    format!("INSERT INTO {table}({fields}) VALUES({values})",
                        table = table,
                        fields = fields.to_sql(&mut 1),
                        values = values.to_sql(&mut 1),
                    );
                quote! {
                    concat!(#query)
                }
            },
            Query::Select { ref filter, get: _get, ref joins, ref limit, ref order, ref table, use_pk: _use_pk } => {
                let where_clause = filter_to_where_clause(filter);
                let order_clause =
                    if has_order_clauses(order) {
                        " ORDER BY "
                    }
                    else {
                        ""
                    };
                let macro_name = Ident::new(format!("tql_{}_field_list", table).as_str(), Span::call_site());
                let joined_fields = joined_fields(&joins, table);
                let check_joins = joins_to_check(&joins);
                let joins = joins_to_tokens(&joins);
                let index = &mut 1;
                let filter = filter.to_tokens(index);
                let order = order.to_sql(&mut 1);
                let limit = limit.to_sql(&mut 1);
                quote_spanned! { Span::call_site() => {
                    #check_joins
                    concat!("SELECT ", #macro_name!() #joined_fields, " FROM ", #table, #joins, #where_clause, #filter,
                        #order_clause, #order, #limit)
                }}
            },
            Query::Update { ref assignments, ref filter, ref table, use_pk: _use_pk } => {
                let where_clause = filter_to_where_clause(filter);
                let index = &mut 1;
                let assignments = assignments.to_sql(index);
                let filter = filter.to_tokens(index);
                quote! {
                    concat!("UPDATE ", #table, " SET ", #assignments, #where_clause, #filter)
                }
            },
        }
    }
}

fn string_token(string: &str) -> Tokens {
    let expr = string_literal(string);
    quote! {
        #expr
    }
}

impl ToSql for RelationalOperator {
    fn to_sql(&self, _index: &mut usize) -> String {
        match *self {
            RelationalOperator::Equal => "=",
            RelationalOperator::LesserThan => "<",
            RelationalOperator::LesserThanEqual => "<=",
            RelationalOperator::NotEqual => "<>",
            RelationalOperator::GreaterThan => ">=",
            RelationalOperator::GreaterThanEqual => ">",
        }.to_string()
    }
}

pub fn fields_to_sql(fields: &[TypedField]) -> Tokens {
    let fields = fields.iter()
        .map(|field| {
             let ident = field.identifier.to_sql(&mut 1);
             let typ = &field.typ;
             quote! {
                 #ident, " ", #typ
             }
        });
    sep_by(fields, ", ")
}

/// Convert a `FilterExpression` to either " WHERE " or the empty string if there are no filters.
fn filter_to_where_clause(filter: &FilterExpression) -> &str {
    match *filter {
        FilterExpression::Filter(_) | FilterExpression::Filters(_) | FilterExpression::NegFilter(_) | FilterExpression::ParenFilter(_) | FilterExpression::FilterValue(_) => " WHERE ",
        FilterExpression::NoFilters => "",
    }
}

fn has_order_clauses(orders: &[Order]) -> bool {
    for order in orders {
        if let Order::NoOrder = *order {
            continue;
        }
        return true;
    }
    false
}

// TODO: check if special characters (\n, \t, …) should be escaped.

/// Escape the character '.
fn escape(string: String) -> String {
    string.replace("'", "''")
}

pub fn type_to_sql(typ: &Type, nullable: bool) -> Tokens {
    let sql_type =
        match *typ {
            Type::Bool => "BOOLEAN",
            Type::ByteString => "BYTEA",
            Type::I8 | Type::Char => "CHARACTER(1)",
            Type::Custom(ref related_table_name) => {
                let pk_macro_name = Ident::new(&format!("tql_{}_primary_key_field", related_table_name),
                    Span::call_site());
                return quote! {
                    "INTEGER REFERENCES ", #related_table_name, "(", #pk_macro_name!(), ") NOT NULL"
                };
                // NOTE: if the field type is not an SQL table, an error is thrown.
            },
            Type::F32 => "REAL",
            Type::F64 => "DOUBLE PRECISION",
            Type::Generic => "", // TODO: document why this is empty.
            Type::I16 => "SMALLINT",
            Type::I32 => "INTEGER",
            Type::I64 => "BIGINT",
            Type::LocalDateTime => "TIMESTAMP WITH TIME ZONE",
            Type::NaiveDate => "DATE",
            Type::NaiveDateTime => "TIMESTAMP",
            Type::NaiveTime => "TIME",
            Type::Nullable(ref typ) => {
                let sql = type_to_sql(&*typ, true);
                return quote! {
                    #sql
                };
            },
            Type::Serial => "INTEGER PRIMARY KEY",
            Type::String => "CHARACTER VARYING",
            Type::UnsupportedType(_) => "", // TODO: should panic. TODO: document why.
            Type::UtcDateTime => "TIMESTAMP WITH TIME ZONE",
        };

    let expr = string_literal(sql_type);
    if nullable {
        quote! {
            #expr
        }
    }
    else {
        quote! {
            #expr, " NOT NULL"
        }
    }
}
