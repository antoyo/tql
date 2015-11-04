//! Abstract syntax tree for SQL generation.

use syntax::ast::Expr;
use syntax::codemap::Spanned;
use syntax::ptr::P;

use state::singleton;
use types::Type;

pub type Expression = P<Expr>;
pub type FieldList = Vec<Identifier>;
pub type Groups = Vec<Identifier>;
pub type Identifier = String;

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
    };
}

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
#[derive(Clone, Debug)]
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

/// `Assignment` for use in SQL Update `Query`.
#[derive(Debug)]
pub struct Assignment {
    pub identifier: Identifier,
    pub value: Expression,
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
                let tables = singleton();
                // NOTE: At this stage (code generation), the table and the field exist, hence unwrap().
                let table = tables.get(table).unwrap();
                if let FilterValue::Identifier(ref identifier) = filter.operand1 {
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
