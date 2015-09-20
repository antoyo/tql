//! A module providing an Abstract Syntax Tree for SQL queries.

use syntax::ast::Expr;
use syntax::ptr::P;

pub mod convert;

pub type Identifier = String;
pub type Expression = P<Expr>;

/// `Filter` for SQL `Query` (WHERE clause).
#[derive(Debug)]
pub struct Filter {
    /// The field from the SQL table to be compared to `operand2`.
    pub operand1: Identifier,
    /// The `operator` used to compare `operand1` to `operand2`.
    pub operator: Operator,
    /// The expression to be compared to `operand1`.
    pub operand2: Expression,
}

/// `Operator` to be used in a `Filter`.
#[derive(Debug)]
pub enum Operator {
    And,
    Or,
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
    CreateTable,
    Delete,
    Insert,
    Select{filter: Option<Filter>, table: String},
    Update,
}
