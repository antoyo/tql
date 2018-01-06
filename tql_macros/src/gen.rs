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

//! The PostgreSQL code generator.

use std::str::from_utf8;

use syn::{Expr, Ident, Lit};

use ast::{
    Aggregate,
    AggregateFilter,
    AggregateFilterExpression,
    AggregateFilters,
    Assignment,
    AssignementOperator,
    Expression,
    FieldList,
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
use sql::escape;
use state::get_primary_key_field_by_table_name;

/// Macro used to generate a ToSql implementation for a filter (for use in WHERE or HAVING).
macro_rules! filter_to_sql {
    ( $name:ident ) => {
        impl ToSql for $name {
            fn to_sql(&self) -> String {
                self.operand1.to_sql() + " " +
                    &self.operator.to_sql() + " " +
                    &self.operand2.to_sql()
            }
        }
    };
}

/// Macro used to generate a ToSql implementation for a filter expression.
macro_rules! filter_expression_to_sql {
    ( $name:ident ) => {
        impl ToSql for $name {
            fn to_sql(&self) -> String {
                match *self {
                    $name::Filter(ref filter) => filter.to_sql(),
                    $name::Filters(ref filters) => filters.to_sql(),
                    $name::NegFilter(ref filter) =>
                        "NOT ".to_string() +
                        &filter.to_sql(),
                    $name::NoFilters => "".to_string(),
                    $name::ParenFilter(ref filter) =>
                        "(".to_string() +
                        &filter.to_sql() +
                        ")",
                    $name::FilterValue(ref filter_value) => filter_value.node.to_sql(),
                }
            }
        }
    };
}

/// Macro used to generate a ToSql implementation for a slice.
macro_rules! slice_to_sql {
    ( $name:ty, $sep:expr ) => {
        impl ToSql for [$name] {
            fn to_sql(&self) -> String {
                self.iter().map(ToSql::to_sql).collect::<Vec<_>>().join($sep)
            }
        }
    };
}

/// A generic trait for converting a value to SQL.
pub trait ToSql {
    fn to_sql(&self) -> String;
}

impl ToSql for Aggregate {
    fn to_sql(&self) -> String {
        // TODO: do not use CAST when this is in a HAVING clause.
        // TODO: do not hard-code the type.
        "CAST(".to_string() + &self.function.to_sql() + "(" + &self.field.expect("Aggregate field").to_sql()
            + ") AS INT)"
    }
}

slice_to_sql!(Aggregate, ", ");

filter_to_sql!(AggregateFilter);

filter_expression_to_sql!(AggregateFilterExpression);

filter_to_sql!(AggregateFilters);

impl ToSql for Assignment {
    fn to_sql(&self) -> String {
        let identifier = self.identifier.expect("Assignment identifier").to_sql();
        if let AssignementOperator::Equal = self.operator.node {
            identifier + &self.operator.node.to_sql() + &self.value.to_sql()
        }
        else {
            identifier.clone() +
                &self.operator.node.to_sql().replace("{}", &identifier) +
                &self.value.to_sql()
        }
    }
}

slice_to_sql!(Assignment, ", ");

impl ToSql for AssignementOperator {
    fn to_sql(&self) -> String {
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
    fn to_sql(&self) -> String {
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
            _ => "?".to_string(),
        }
    }
}

slice_to_sql!(Expression, ", ");

impl ToSql for FieldList {
    fn to_sql(&self) -> String {
        self.join(", ")
    }
}

filter_to_sql!(Filter);

filter_expression_to_sql!(FilterExpression);

filter_to_sql!(Filters);

impl ToSql for Ident {
    fn to_sql(&self) -> String {
        self.to_string()
    }
}

impl ToSql for FilterValue {
    fn to_sql(&self) -> String {
        match *self {
            FilterValue::Identifier(ref identifier) => identifier.to_sql(),
            FilterValue::MethodCall(MethodCall { ref arguments, ref object_name, ref template, ..  }) => {
                // In the template, $0 represents the object identifier and $1, $2, ... the
                // arguments.
                let mut sql = template.replace("$0", &object_name.to_string());
                let mut index = 1;
                for argument in arguments {
                    sql = sql.replace(&format!("${}", index), &argument.to_sql());
                    index += 1;
                }
                sql
            },
            FilterValue::None => unreachable!("FilterValue::None in FilterValue::to_sql()"),
        }
    }
}

impl ToSql for Join {
    fn to_sql(&self) -> String {
        " INNER JOIN ".to_string() + &self.joined_table +
            " ON " + &self.base_table + "." + &self.base_field + " = "
            + &self.joined_table + "." + &self.joined_field
    }
}

slice_to_sql!(Join, " ");

impl ToSql for Identifier {
    fn to_sql(&self) -> String {
        self.clone()
    }
}

impl ToSql for Limit {
    fn to_sql(&self) -> String {
        match *self {
            EndRange(ref expression) => " LIMIT ".to_string() + &expression.to_sql(),
            Index(ref expression) =>
                " OFFSET ".to_string() + &expression.to_sql() +
                " LIMIT 1",
            LimitOffset(ref expression1, ref expression2) =>
                " OFFSET ".to_string() + &expression2.to_sql() +
                " LIMIT " + &expression1.to_sql(),
            NoLimit => "".to_string(),
            Range(ref expression1, ref expression2) =>
                " OFFSET ".to_string() + &expression1.to_sql() +
                " LIMIT " + &expression2.to_sql(),
            StartRange(ref expression) => " OFFSET ".to_string() + &expression.to_sql(),
        }
    }
}

impl ToSql for LogicalOperator {
    fn to_sql(&self) -> String {
        match *self {
            LogicalOperator::And => "AND",
            LogicalOperator::Not => "NOT",
            LogicalOperator::Or => "OR",
        }.to_string()
    }
}

impl ToSql for Order {
    fn to_sql(&self) -> String {
        match *self {
            Order::Ascending(ref field) => field.to_sql(),
            Order::Descending(ref field) => field.to_sql() + " DESC",
        }
    }
}

slice_to_sql!(Order, ", ");

/// Convert a whole `Query` to SQL.
impl ToSql for Query {
    fn to_sql(&self) -> String {
        match *self {
            Query::Aggregate{ref aggregates, ref aggregate_filter, ref filter, ref groups, ref joins, ref table} => {
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
                replace_placeholder(format!("SELECT {aggregates} FROM {table_name}{joins}{where_clause}{filter}{group_clause}{groups}{having_clause}{aggregate_filter}",
                                            aggregates = aggregates.to_sql(),
                                            table_name = table,
                                            joins = joins.to_sql(),
                                            where_clause = where_clause,
                                            filter = filter.to_sql(),
                                            group_clause = group_clause,
                                            groups = groups.to_sql(),
                                            having_clause = having_clause,
                                            aggregate_filter = aggregate_filter.to_sql())
                                    )
            },
            Query::CreateTable { ref fields, ref table } => {
                format!("CREATE TABLE {table} ({fields})",
                    table = table,
                    fields = fields.to_sql()
                )
            },
            Query::Delete { ref filter, ref table } => {
                let where_clause = filter_to_where_clause(filter);
                replace_placeholder(format!("DELETE FROM {table}{where_clause}{filter}",
                                            table = table,
                                            where_clause = where_clause,
                                            filter = filter.to_sql()
                                           )
                                   )
            },
            Query::Drop { ref table } => {
                format!("DROP TABLE {table}", table = table)
            },
            Query::Insert { ref assignments, ref table } => {
                let fields: Vec<_> = assignments.iter().map(|assign|
                    assign.identifier.expect("Assignment identifier").to_sql()).collect();
                let values: Vec<_> = assignments.iter().map(|assign| assign.value.to_sql()).collect();
                // Add the SQL code to get the inserted primary key.
                // TODO: what to do when there is no primary key?
                let return_value = get_primary_key_field_by_table_name(table)
                    .map_or("".to_string(), |primary_key| " RETURNING ".to_string() + &primary_key);
                replace_placeholder(format!("INSERT INTO {table}({fields}) VALUES({values}){return_value}",
                        table = table,
                        fields = fields.to_sql(),
                        values = values.to_sql(),
                        return_value = return_value
                    ))
            },
            Query::Select{ref fields, ref filter, ref joins, ref limit, ref order, ref table} => {
                let where_clause = filter_to_where_clause(filter);
                let order_clause =
                    if !order.is_empty() {
                        " ORDER BY "
                    }
                    else {
                        ""
                    };
                replace_placeholder(format!("SELECT {fields} FROM {table}{joins}{where_clause}{filter}{order_clause}{order}{limit}",
                                            fields = fields.to_sql(),
                                            table = table,
                                            joins = joins.to_sql(),
                                            where_clause = where_clause,
                                            filter = filter.to_sql(),
                                            order_clause = order_clause,
                                            order = order.to_sql(),
                                            limit = limit.to_sql()
                                           )
                                   )
            },
            Query::Update { ref assignments, ref filter, ref table } => {
                let where_clause = filter_to_where_clause(filter);
                replace_placeholder(format!("UPDATE {table} SET {assignments}{where_clause}{filter}",
                                            table = table,
                                            assignments = assignments.to_sql(),
                                            where_clause = where_clause,
                                            filter = filter.to_sql()
                                           )
                                   )
            },
        }
    }
}

impl ToSql for RelationalOperator {
    fn to_sql(&self) -> String {
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

impl ToSql for TypedField {
    fn to_sql(&self) -> String {
        self.identifier.to_sql() + " " + &self.typ
    }
}

slice_to_sql!(TypedField, ", ");

/// Convert a `FilterExpression` to either " WHERE " or the empty string if there are no filters.
fn filter_to_where_clause(filter: &FilterExpression) -> &str {
    match *filter {
        FilterExpression::Filter(_) | FilterExpression::Filters(_) | FilterExpression::NegFilter(_) | FilterExpression::ParenFilter(_) | FilterExpression::FilterValue(_) => " WHERE ",
        FilterExpression::NoFilters => "",
    }
}

// TODO: find a better way to write the symbols ($1, $2, …) in the query.
/// Replace the placeholders `{}` by $# by # where # is the index of the placeholder.
fn replace_placeholder(string: String) -> String {
    let mut result = "".to_string();
    let mut in_string = false;
    let mut skip_next = false;
    let mut index = 1;
    for character in string.chars() {
        if character == '?' && !in_string {
            result.push('$');
            result.push_str(&index.to_string());
            index = index + 1;
        }
        else {
            if character == '\\' {
                skip_next = true;
            }
            else if character == '\'' && !skip_next {
                skip_next = false;
                in_string = !in_string;
            }
            else {
                skip_next = false;
            }
            result.push(character);
        }
    }
    result
}
