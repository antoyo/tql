//! Abstract syntax tree for SQL generation.

use syntax::ast::Expr;
use syntax::ptr::P;

pub type Expression = P<Expr>;
pub type FieldList = Vec<Identifier>;
pub type Identifier = String;
pub type Type = String;

/// `Assignment` for use in SQL Update `Query`.
#[derive(Debug)]
pub struct Assignment {
    pub identifier: Identifier,
    pub value: Expression,
}

/// `Filter` for SQL `Query` (WHERE clause).
#[derive(Debug)]
pub struct Filter {
    /// The field from the SQL table to be compared to `operand2`.
    // TODO: aussi permettre les appels de méthode.
    pub operand1: Identifier,
    /// The `operator` used to compare `operand1` to `operand2`.
    pub operator: RelationalOperator,
    /// The expression to be compared to `operand1`.
    pub operand2: Expression,
}

/// Either a single `Filter`, `Filters` or `NoFilters`.
#[derive(Debug)]
pub enum FilterExpression {
    // TODO: aussi permettre les appels de méthode.
    Filter(Filter),
    Filters(Filters),
    NoFilters,
}

/// A `Filters` is used to combine `FilterExpression`s with a `LogicalOperator`.
#[derive(Debug)]
pub struct Filters {
    /// The `FilterExpression` to be combined with `operand2`.
    pub operand1: Box<FilterExpression>,
    /// The `LogicalOperator` used to combine the `FilterExpression`s.
    pub operator: LogicalOperator,
    /// The `FilterExpression` to be combined with `operand1`.
    pub operand2: Box<FilterExpression>,
}

/// An SQL LIMIT clause.
#[derive(Debug)]
pub enum Limit {
    EndRange(Expression),
    Index(Expression),
    LimitOffset(u64, u64),
    NoLimit,
    Range(Expression, Expression),
    StartRange(Expression),
}

/// `LogicalOperator` to combine `Filter`s.
#[derive(Debug)]
pub enum LogicalOperator {
    And,
    Not,
    Or,
}

/// An SQL ORDER BY clause.
#[derive(Debug)]
pub enum Order {
    Ascending(Identifier),
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
pub enum Query<'a> {
    CreateTable {
        fields: &'a[TypedField],
        table: Identifier,
    },
    Delete {
        filter: FilterExpression,
        table: Identifier,
    },
    Insert {
        fields: FieldList,
        table: Identifier,
    },
    Select {
        fields: FieldList,
        filter: FilterExpression,
        joins: Vec<Identifier>,
        limit: Limit,
        order: Vec<Order>,
        table: Identifier,
    },
    Update {
        assignments: &'a[Assignment],
        filter: FilterExpression,
        table: Identifier,
    },
}

/// The type of the query.
pub enum QueryType {
    SelectOne,
    SelectMulti,
}

/// An SQL field with its type.
#[derive(Debug)]
pub struct TypedField {
    identifier: Identifier,
    typ: Type,
}

/// Get the query type.
pub fn query_type(query: &Query) -> QueryType {
    QueryType::SelectMulti
}
