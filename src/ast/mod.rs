//! A module providing an Abstract Syntax Tree for SQL queries.

use syntax::ast::Expr;
use syntax::ptr::P;

pub mod convert;

pub type Expression = P<Expr>;
pub type Identifier = String;
pub type FieldList<'a> = &'a[&'a Identifier];
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

/// `LogicalOperator` to combine `Filter`s.
#[derive(Debug)]
pub enum LogicalOperator {
    And,
    Not,
    Or,
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

/// An SQL ORDER BY clause.
#[derive(Debug)]
pub enum Order {
    Ascending(Identifier),
    Descending(Identifier),
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
        fields: FieldList<'a>,
        table: Identifier,
    },
    Select {
        fields: FieldList<'a>,
        filter: FilterExpression,
        joins: &'a[Identifier],
        limit: Option<(u32, u32)>,
        order: &'a[Order],
        table: Identifier,
    },
    Update {
        assignments: &'a[Assignment],
        filter: FilterExpression,
        table: Identifier,
    },
}

/// An SQL field with its type.
#[derive(Debug)]
pub struct TypedField {
    identifier: Identifier,
    typ: Type,
}
