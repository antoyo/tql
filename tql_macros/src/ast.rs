//! Abstract syntax tree for SQL generation.

use syntax::ast::Expr;
use syntax::codemap::Spanned;
use syntax::ptr::P;

use state::singleton;
use types::Type;

pub type Expression = P<Expr>;
pub type FieldList = Vec<Identifier>;
pub type Identifier = String;

/// `Aggregate` for une in SQL Aggregate `Query`.
#[derive(Clone, Debug)]
pub struct Aggregate {
    pub field: Identifier,
    pub function: Identifier,
}

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
    pub operand1: RValue,
    /// The `operator` used to compare `operand1` to `operand2`.
    pub operator: RelationalOperator,
    /// The expression to be compared to `operand1`.
    pub operand2: Expression,
}

/// Either a single `Filter`, `Filters` or `NoFilters`.
#[derive(Debug)]
pub enum FilterExpression {
    Filter(Filter),
    Filters(Filters),
    NegFilter(Box<FilterExpression>),
    NoFilters,
    ParenFilter(Box<FilterExpression>),
    RValue(Spanned<RValue>),
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

/// A `Join` with another `table` via a specific `field`.
#[derive(Clone, Debug)]
pub struct Join {
    pub left_field: Identifier,
    pub left_table: Identifier,
    pub right_field: Identifier,
    pub right_table: Identifier,
}

/// An SQL LIMIT clause.
#[derive(Clone, Debug)]
pub enum Limit {
    EndRange(Expression),
    Index(Expression),
    LimitOffset(Expression, Expression),
    NoLimit,
    Range(Expression, Expression),
    StartRange(Expression),
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

/// Either an identifier or a method call.
#[derive(Debug)]
pub enum RValue {
    Identifier(Identifier),
    MethodCall(MethodCall),
}

/// An SQL `Query`.
#[derive(Debug)]
pub enum Query {
    Aggregate {
        aggregates: Vec<Aggregate>,
        filter: FilterExpression,
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
    AggregateOne,
    Exec,
    InsertOne,
    SelectOne,
    SelectMulti,
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
        Query::Aggregate { .. } => QueryType::AggregateOne,
        Query::Insert { .. } => QueryType::InsertOne,
        Query::Select { ref filter, ref limit, ref table, .. } => {
            let mut typ = QueryType::SelectMulti;
            if let FilterExpression::Filter(ref filter) = *filter {
                let tables = singleton();
                // NOTE: At this stage (code generation), the table and the field exist, hence unwrap().
                let table = tables.get(table).unwrap();
                if let RValue::Identifier(ref identifier) = filter.operand1 {
                    if table.get(identifier).unwrap().node == Type::Serial {
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
