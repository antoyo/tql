/*
 * Copyright (C) 2015  Boucher, Antoni <bouanto@zoho.com>
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

//! The PostgreSQL code generator.

use std::str::from_utf8;

use syntax::ast::Expr_::ExprLit;
use syntax::ast::Lit_::{LitBool, LitByte, LitByteStr, LitChar, LitFloat, LitFloatUnsuffixed, LitInt, LitStr};

use ast::{Aggregate, AggregateFilter, AggregateFilterExpression, AggregateFilters, AggregateFilterValue, Assignment, AssignementOperator, Expression, FieldList, Filter, Filters, FilterExpression, FilterValue, Identifier, Join, Limit, LogicalOperator, MethodCall, Order, RelationalOperator, Query, TypedField};
use ast::Limit::{EndRange, Index, LimitOffset, NoLimit, Range, StartRange};
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
                        "NOT ".to_owned() +
                        &filter.to_sql(),
                    $name::NoFilters => "".to_owned(),
                    $name::ParenFilter(ref filter) =>
                        "(".to_owned() +
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
        "CAST(".to_owned() + &self.function.to_sql() + "(" + &self.field.to_sql() + ") AS INT)"
    }
}

slice_to_sql!(Aggregate, ", ");

filter_to_sql!(AggregateFilter);

filter_expression_to_sql!(AggregateFilterExpression);

filter_to_sql!(AggregateFilters);

impl ToSql for AggregateFilterValue {
    fn to_sql(&self) -> String {
        match *self {
            AggregateFilterValue::Sql(ref sql) => sql.clone(),
        }
    }
}

impl ToSql for Assignment {
    fn to_sql(&self) -> String {
        if let AssignementOperator::Equal = self.operator.node {
            self.identifier.to_sql() +
                &self.operator.node.to_sql() +
                &self.value.to_sql()
        }
        else {
            let identifier = self.identifier.to_sql();
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
        }.to_owned()
    }
}

/// Convert a literal expression to its SQL representation.
/// A non-literal is converted to ? for use with query parameters.
impl ToSql for Expression {
    fn to_sql(&self) -> String {
        match self.node {
            ExprLit(ref literal) => {
                match literal.node {
                    LitBool(boolean) => boolean.to_string().to_uppercase(),
                    LitByte(byte) =>
                        "'".to_owned() +
                        &escape((byte as char).to_string()) +
                        "'",
                    // TODO: check if using unwrap() is secure here.
                    LitByteStr(ref bytestring) =>
                        "'".to_owned() +
                        &escape(from_utf8(&bytestring[..]).unwrap().to_owned()) +
                        "'",
                    LitChar(character) =>
                        "'".to_owned() +
                        &escape(character.to_string()) +
                        "'",
                    LitFloat(ref float, _) => float.to_string(),
                    LitFloatUnsuffixed(ref float) => float.to_string(),
                    LitInt(number, _) => number.to_string(),
                    LitStr(ref string, _) =>
                        "'".to_owned() +
                        &escape(string.to_string()) +
                        "'",
                }
            },
            _ => "?".to_owned(),
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

impl ToSql for FilterValue {
    fn to_sql(&self) -> String {
        match *self {
            FilterValue::Identifier(ref identifier) => identifier.to_sql(),
            FilterValue::MethodCall(MethodCall { ref arguments, ref object_name, ref template, ..  }) => {
                // In the template, $0 represents the object identifier and $1, $2, ... the
                // arguments.
                let mut sql = template.replace("$0", object_name);
                let mut index = 1;
                for argument in arguments {
                    sql = sql.replace(&format!("${}", index), &argument.to_sql());
                    index += 1;
                }
                sql
            },
        }
    }
}

impl ToSql for Join {
    fn to_sql(&self) -> String {
        " INNER JOIN ".to_owned() + &self.joined_table +
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
            EndRange(ref expression) => " LIMIT ".to_owned() + &expression.to_sql(),
            Index(ref expression) =>
                " OFFSET ".to_owned() + &expression.to_sql() +
                " LIMIT 1",
            LimitOffset(ref expression1, ref expression2) =>
                " OFFSET ".to_owned() + &expression2.to_sql() +
                " LIMIT " + &expression1.to_sql(),
            NoLimit => "".to_owned(),
            Range(ref expression1, ref expression2) =>
                " OFFSET ".to_owned() + &expression1.to_sql() +
                " LIMIT " + &expression2.to_sql(),
            StartRange(ref expression) => " OFFSET ".to_owned() + &expression.to_sql(),
        }
    }
}

impl ToSql for LogicalOperator {
    fn to_sql(&self) -> String {
        match *self {
            LogicalOperator::And => "AND",
            LogicalOperator::Not => "NOT",
            LogicalOperator::Or => "OR",
        }.to_owned()
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
                let fields: Vec<_> = assignments.iter().map(|assign| assign.identifier.to_sql()).collect();
                let values: Vec<_> = assignments.iter().map(|assign| assign.value.to_sql()).collect();
                // Add the SQL code to get the inserted primary key.
                let return_value = get_primary_key_field_by_table_name(table)
                    .map(|primary_key| " RETURNING ".to_owned() + &primary_key)
                    .unwrap_or("".to_owned()); // TODO: what to do whene there is no primary key?
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
        }.to_owned()
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
    let mut result = "".to_owned();
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
