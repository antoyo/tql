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

//! Abstract syntax tree for SQL generation.

use std::fmt::{Display, Error, Formatter};

use syntax::ast::Expr;
use syntax::codemap::Spanned;
use syntax::ptr::P;

use state::tables_singleton;
use types::Type;

pub type Expression = P<Expr>;
pub type FieldList = Vec<Identifier>;
pub type Groups = Vec<Identifier>;
pub type Identifier = String;

/// Macro generating a struct for a filter type.
macro_rules! filter {
    ( $name:ident, $ty:ty ) => {
        #[derive(Debug)]
        pub struct $name {
            /// The filter value to be compared to `operand2`.
            pub operand1: $ty,
            /// The `operator` used to compare `operand1` to `operand2`.
            pub operator: RelationalOperator,
            /// The expression to be compared to `operand1`.
            pub operand2: Expression,
        }
    };
}

/// Macro generating an enum for a filter expression type.
macro_rules! filter_expression {
    ( $name:ident, $filter_name:ty, $filters_name:ty, $filter_expression_name:ty, $filter_value_name:ty ) => {
        #[derive(Debug)]
        pub enum $name {
            Filter($filter_name),
            Filters($filters_name),
            NegFilter(Box<$filter_expression_name>),
            NoFilters,
            ParenFilter(Box<$filter_expression_name>),
            FilterValue(Spanned<$filter_value_name>),
        }

        impl Default for $name {
            fn default() -> $name {
                $name::NoFilters
            }
        }
    };
}

/// Macro generating a struct for a filters type.
macro_rules! filters {
    ( $name:ident, $ty:ty ) => {
        #[derive(Debug)]
        pub struct $name {
            /// The `T` to be combined with `operand2`.
            pub operand1: Box<$ty>,
            /// The `LogicalOperator` used to combine the `FilterExpression`s.
            pub operator: LogicalOperator,
            /// The `T` to be combined with `operand1`.
            pub operand2: Box<$ty>,
        }
    };
}

/// `Aggregate` for une in SQL Aggregate `Query`.
#[derive(Clone, Debug, Default)]
pub struct Aggregate {
    pub field: Identifier,
    pub function: Identifier,
    pub result_name: Identifier,
}

/// `AggregateFilter` for SQL `Query` (HAVING clause).
filter!(AggregateFilter, AggregateFilterValue);

/// Aggregate filter expression.
filter_expression!(AggregateFilterExpression, AggregateFilter, AggregateFilters, AggregateFilterExpression, AggregateFilterValue);

/// A `Filters` is used to combine `AggregateFilterExpression`s with a `LogicalOperator`.
filters!(AggregateFilters, AggregateFilterExpression);

/// Either an identifier or a method call.
#[derive(Debug)]
pub enum AggregateFilterValue {
    Sql(String),
}

/// `Assignment` for use in SQL Insert and Update `Query`.
#[derive(Debug)]
pub struct Assignment {
    pub identifier: Identifier,
    pub operator: Spanned<AssignementOperator>,
    pub value: Expression,
}

/// `AssignementOperator` for use in SQL Insert and Update `Query`.
#[derive(Debug, PartialEq)]
pub enum AssignementOperator {
    Add,
    Divide,
    Equal,
    Modulo,
    Mul,
    Sub,
}

impl Display for AssignementOperator {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), Error> {
        let op =
            match *self {
                AssignementOperator::Add => "+=",
                AssignementOperator::Divide => "/=",
                AssignementOperator::Equal => "=",
                AssignementOperator::Modulo => "%=",
                AssignementOperator::Mul => "*=",
                AssignementOperator::Sub => "-=",
            };
        write!(formatter, "{}", op).unwrap();
        Ok(())
    }
}

/// `Filter` for SQL `Query` (WHERE clause).
filter!(Filter, FilterValue);

/// Either a single `Filter`, `Filters`, `NegFilter`, `NoFilters`, `ParenFilter` or a `FilterValue`.
filter_expression!(FilterExpression, Filter, Filters, FilterExpression, FilterValue);

/// A `Filters` is used to combine `FilterExpression`s with a `LogicalOperator`.
filters!(Filters, FilterExpression);

/// Either an identifier or a method call.
#[derive(Debug)]
pub enum FilterValue {
    Identifier(Identifier),
    MethodCall(MethodCall),
}

/// A `Join` with another `joined_table` via a specific `joined_field`.
#[derive(Clone, Debug, Default)]
pub struct Join {
    pub base_field: Identifier,
    pub base_table: Identifier,
    pub joined_field: Identifier,
    pub joined_table: Identifier,
}

/// An SQL LIMIT clause.
#[derive(Clone, Debug)]
pub enum Limit {
    /// [..end]
    EndRange(Expression),
    /// [index]
    Index(Expression),
    /// Not created from a query. It is converted from a `Range`.
    LimitOffset(Expression, Expression),
    /// No limit was specified.
    NoLimit,
    /// [start..end]
    Range(Expression, Expression),
    /// [start..]
    StartRange(Expression),
}

impl Default for Limit {
    fn default() -> Limit {
        Limit::NoLimit
    }
}

/// `LogicalOperator` to combine `Filter`s.
#[derive(Debug, PartialEq)]
pub enum LogicalOperator {
    And,
    Not,
    Or,
}

/// A method call is an abstraction of SQL function call.
#[derive(Debug)]
pub struct MethodCall {
    pub arguments: Vec<Expression>,
    pub method_name: Identifier,
    pub object_name: Identifier,
    pub template: String,
}

/// An SQL ORDER BY clause.
#[derive(Debug)]
pub enum Order {
    /// Comes from `sort(field)`.
    Ascending(Identifier),
    /// Comes from `sort(-field)`.
    Descending(Identifier),
}

/// `RelationalOperator` to be used in a `Filter`.
#[derive(Debug)]
pub enum RelationalOperator {
    Equal,
    LesserThan,
    LesserThanEqual,
    NotEqual,
    GreaterThan,
    GreaterThanEqual,
}

/// An SQL `Query`.
#[derive(Debug)]
pub enum Query {
    Aggregate {
        aggregates: Vec<Aggregate>,
        aggregate_filter: AggregateFilterExpression,
        filter: FilterExpression,
        groups: Groups,
        joins: Vec<Join>,
        table: Identifier,
    },
    CreateTable {
        fields: Vec<TypedField>,
        table: Identifier,
    },
    Delete {
        filter: FilterExpression,
        table: Identifier,
    },
    Drop {
        table: Identifier,
    },
    Insert {
        assignments: Vec<Assignment>,
        table: Identifier,
    },
    Select {
        fields: FieldList,
        filter: FilterExpression,
        joins: Vec<Join>,
        limit: Limit,
        order: Vec<Order>,
        table: Identifier,
    },
    Update {
        assignments: Vec<Assignment>,
        filter: FilterExpression,
        table: Identifier,
    },
}

/// The type of the query.
pub enum QueryType {
    AggregateMulti,
    AggregateOne,
    Exec,
    InsertOne,
    SelectMulti,
    SelectOne,
}

/// An SQL field with its type.
#[derive(Debug)]
pub struct TypedField {
    pub identifier: Identifier,
    pub typ: String,
}

/// Get the query table name.
pub fn query_table(query: &Query) -> Identifier {
    let table_name =
        match *query {
            Query::Aggregate { ref table, .. } => table,
            Query::CreateTable { ref table, .. } => table,
            Query::Delete { ref table, .. } => table,
            Query::Drop { ref table, .. } => table,
            Query::Insert { ref table, .. } => table,
            Query::Select { ref table, .. } => table,
            Query::Update { ref table, .. } => table,
        };
    table_name.clone()
}

/// Get the query type.
pub fn query_type(query: &Query) -> QueryType {
    match *query {
        Query::Aggregate { ref groups, .. } => {
            if !groups.is_empty() {
                QueryType::AggregateMulti
            }
            else {
                QueryType::AggregateOne
            }
        },
        Query::Insert { .. } => QueryType::InsertOne,
        Query::Select { ref filter, ref limit, ref table, .. } => {
            let mut typ = QueryType::SelectMulti;
            if let FilterExpression::Filter(ref filter) = *filter {
                let tables = tables_singleton();
                // NOTE: At this stage (code generation), the table and the field exist, hence unwrap().
                let table = tables.get(table).unwrap();
                if let FilterValue::Identifier(ref identifier) = filter.operand1 {
                    if table.fields.get(identifier).unwrap().node == Type::Serial {
                        typ = QueryType::SelectOne;
                    }
                }
            }
            if let Limit::Index(_) = *limit {
                typ = QueryType::SelectOne;
            }
            typ
        },
        Query::CreateTable { .. } | Query::Delete { .. } | Query::Drop { .. } | Query::Update { .. } => QueryType::Exec,
    }
}
