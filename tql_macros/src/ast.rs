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

//! Abstract syntax tree for SQL generation.

use std::fmt::{Display, Error, Formatter};

use syn::{
    self,
    Block,
    Expr,
    ExprAddrOf,
    ExprArray,
    ExprAssign,
    ExprAssignOp,
    ExprBinary,
    ExprBlock,
    ExprBox,
    ExprBreak,
    ExprCall,
    ExprCast,
    ExprCatch,
    ExprClosure,
    ExprContinue,
    ExprField,
    ExprForLoop,
    ExprGroup,
    ExprIf,
    ExprIfLet,
    ExprIndex,
    ExprInPlace,
    ExprKind,
    ExprLoop,
    ExprMatch,
    ExprMethodCall,
    ExprParen,
    ExprPath,
    ExprRange,
    ExprRepeat,
    ExprRet,
    ExprStruct,
    ExprTry,
    ExprTup,
    ExprTupField,
    ExprType,
    ExprUnary,
    ExprUnsafe,
    ExprWhile,
    ExprWhileLet,
    ExprYield,
    GenericArgument,
    Ident,
    Item,
    ItemStruct,
    Lit,
    Macro,
    Path,
    RangeLimits,
    Span,
    TypePath,
    TypeReference,
};
#[cfg(feature = "unstable")]
use syn::AngleBracketedGenericArguments;
#[cfg(feature = "unstable")]
use syn::PathArguments::AngleBracketed;

use state::tables_singleton;
use types::Type;

pub type Expression = Expr;
pub type FieldList = Vec<Identifier>;
pub type Groups = Vec<Identifier>;
pub type Identifier = String;

/// `Aggregate` for une in SQL Aggregate `Query`.
#[derive(Clone, Debug, Default)]
pub struct Aggregate {
    pub field: Option<Ident>,
    pub function: Identifier,
    pub result_name: Option<Ident>,
}

/// `AggregateFilter` for SQL `Query` (HAVING clause).
#[derive(Debug)]
pub struct AggregateFilter {
    /// The filter value to be compared to `operand2`.
    pub operand1: Aggregate,
    /// The `operator` used to compare `operand1` to `operand2`.
    pub operator: RelationalOperator,
    /// The expression to be compared to `operand1`.
    pub operand2: Expression,
}

/// Aggregate filter expression.
#[derive(Debug)]
pub enum AggregateFilterExpression {
    Filter(AggregateFilter),
    Filters(AggregateFilters),
    NegFilter(Box<AggregateFilterExpression>),
    NoFilters,
    ParenFilter(Box<AggregateFilterExpression>),
    FilterValue(WithSpan<Aggregate>),
}

impl Default for AggregateFilterExpression {
    fn default() -> Self {
        AggregateFilterExpression::NoFilters
    }
}

/// A `Filters` is used to combine `AggregateFilterExpression`s with a `LogicalOperator`.
#[derive(Debug)]
pub struct AggregateFilters {
    /// The `T` to be combined with `operand2`.
    pub operand1: Box<AggregateFilterExpression>,
    /// The `LogicalOperator` used to combine the `FilterExpression`s.
    pub operator: LogicalOperator,
    /// The `T` to be combined with `operand1`.
    pub operand2: Box<AggregateFilterExpression>,
}

/// `Assignment` for use in SQL Insert and Update `Query`.
#[derive(Debug)]
pub struct Assignment {
    pub identifier: Option<Ident>,
    pub operator: WithSpan<AssignementOperator>,
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
#[derive(Debug)]
pub struct Filter {
    /// The filter value to be compared to `operand2`.
    pub operand1: FilterValue,
    /// The `operator` used to compare `operand1` to `operand2`.
    pub operator: RelationalOperator,
    /// The expression to be compared to `operand1`.
    pub operand2: Expression,
}

/// Either a single `Filter`, `Filters`, `NegFilter`, `NoFilters`, `ParenFilter` or a `FilterValue`.
#[derive(Debug)]
pub enum FilterExpression {
    Filter(Filter),
    Filters(Filters),
    NegFilter(Box<FilterExpression>),
    NoFilters,
    ParenFilter(Box<FilterExpression>),
    FilterValue(WithSpan<FilterValue>),
}

impl Default for FilterExpression {
    fn default() -> Self {
        FilterExpression::NoFilters
    }
}

/// A `Filters` is used to combine `FilterExpression`s with a `LogicalOperator`.
#[derive(Debug)]
pub struct Filters {
    /// The `T` to be combined with `operand2`.
    pub operand1: Box<FilterExpression>,
    /// The `LogicalOperator` used to combine the `FilterExpression`s.
    pub operator: LogicalOperator,
    /// The `T` to be combined with `operand1`.
    pub operand2: Box<FilterExpression>,
}

/// Either an identifier or a method call.
#[derive(Debug)]
pub enum FilterValue {
    None,
    Identifier(Ident),
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
    pub object_name: Ident,
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
                    if table.fields.get(identifier).unwrap().ty.node == Type::Serial {
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

#[derive(Debug)]
pub struct WithSpan<T> {
    pub node: T,
    pub span: Span,
}

pub fn expr_span(expr: &Expr) -> Span {
    match expr.node {
        ExprKind::AddrOf(ExprAddrOf { ref expr, .. }) => expr_span(expr),
        ExprKind::Array(ExprArray { ref bracket_token, .. }) => bracket_token.0,
        ExprKind::Assign(ExprAssign { ref left, .. }) => expr_span(left),
        ExprKind::AssignOp(ExprAssignOp { ref left, .. }) => expr_span(left),
        // FIXME: should take the hi of `right` for Binary.
        ExprKind::Binary(ExprBinary { ref left, .. }) => expr_span(left),
        ExprKind::Block(ExprBlock { block: Block { ref brace_token, .. }}) => brace_token.0,
        ExprKind::Box(ExprBox { ref box_token, .. }) => box_token.0,
        ExprKind::Break(ExprBreak { ref break_token, .. }) => break_token.0,
        ExprKind::Call(ExprCall { ref func, .. }) => expr_span(func),
        ExprKind::Cast(ExprCast { ref expr, .. }) => expr_span(expr),
        ExprKind::Catch(ExprCatch { ref do_token, .. }) => do_token.0,
        ExprKind::Closure(ExprClosure { ref or1_token, .. }) => or1_token.0[0],
        ExprKind::Continue(ExprContinue { ref continue_token, .. }) => continue_token.0,
        ExprKind::Field(ExprField { ref expr, .. }) => expr_span(expr),
        ExprKind::ForLoop(ExprForLoop { ref for_token, .. }) => for_token.0,
        ExprKind::Group(ExprGroup { ref group_token, .. }) => group_token.0,
        ExprKind::If(ExprIf { ref if_token, .. }) => if_token.0,
        ExprKind::IfLet(ExprIfLet { ref if_token, .. }) => if_token.0,
        ExprKind::Index(ExprIndex { ref expr, .. }) => expr_span(expr),
        ExprKind::InPlace(ExprInPlace { ref place, .. }) => expr_span(place),
        ExprKind::Lit(Lit { span, .. }) => span,
        ExprKind::Loop(ExprLoop { ref loop_token, .. }) => loop_token.0,
        ExprKind::Macro(Macro { ref path, .. }) => path_span(path),
        ExprKind::Match(ExprMatch { ref match_token, .. }) => match_token.0,
        ExprKind::MethodCall(ExprMethodCall { ref expr, .. }) => {
            expr_span(expr)
        },
        ExprKind::Paren(ExprParen { ref paren_token, .. }) => paren_token.0,
        ExprKind::Path(ExprPath { ref path, .. }) => path_span(path),
        ExprKind::Range(ExprRange { ref from, ref limits, .. }) => {
            from.as_ref().map(|expr| expr_span(&*expr))
                .unwrap_or_else(||
                    match *limits {
                        RangeLimits::Closed(dots) => dots.0[0],
                        RangeLimits::HalfOpen(dots) => dots.0[0],
                    }
                )
        },
        ExprKind::Repeat(ExprRepeat { ref bracket_token, .. }) => bracket_token.0,
        ExprKind::Ret(ExprRet { ref return_token, .. }) => return_token.0,
        ExprKind::Struct(ExprStruct { ref path, .. }) => path_span(path),
        ExprKind::Try(ExprTry { ref expr, .. }) => expr_span(expr),
        ExprKind::Tup(ExprTup { ref paren_token, .. }) => paren_token.0,
        ExprKind::TupField(ExprTupField { ref expr, .. }) => expr_span(expr),
        ExprKind::Type(ExprType { ref expr, .. }) => expr_span(expr),
        ExprKind::Unary(ExprUnary { ref expr, .. }) => expr_span(expr),
        ExprKind::Unsafe(ExprUnsafe { block: Block { ref brace_token, .. }, .. }) => brace_token.0,
        ExprKind::While(ExprWhile { ref while_token, .. }) => while_token.0,
        ExprKind::WhileLet(ExprWhileLet { ref while_token, .. }) => while_token.0,
        ExprKind::Yield(ExprYield { ref yield_token, .. }) => yield_token.0,
    }
}

pub fn arg_span(arg: &GenericArgument) -> Span {
    match *arg {
        GenericArgument::Type(ref typ) => type_span(typ),
        _ => unimplemented!("Arg: {:?}", arg),
    }
}

pub fn item_span(item: &Item) -> Span {
    match *item {
        Item::Struct(ItemStruct { ref ident, .. }) => ident.span,
        _ => unimplemented!("Item {:?}", item),
    }
}

pub fn path_span(path: &Path) -> Span {
    path.segments.first().expect("first segment in path").item().ident.span
}

pub fn type_span(typ: &syn::Type) -> Span {
    match *typ {
        syn::Type::Path(TypePath { ref path, .. }) => path_span(path),
        syn::Type::Reference(TypeReference { ref and_token, .. }) => and_token.0[0],
        _ => unimplemented!("Type: {:?}", typ),
    }
}

#[cfg(feature = "unstable")]
pub fn generic_arg_span(typ: &syn::Type) -> Span {
    match *typ {
        syn::Type::Path(TypePath { ref path, .. }) => {
            let arguments = &path.segments.first().expect("first segment in path").item().arguments;
            if let AngleBracketed(AngleBracketedGenericArguments { ref args, .. }) = *arguments {
                if let GenericArgument::Type(ref ty) = **args.first().expect("first generic argument").item() {
                    type_span(ty)
                }
                else {
                    panic!("Expecting type generic argument");
                }
            }
            else {
                panic!("Expecting generic argument");
            }
        },
        _ => unimplemented!("Generic Arg: {:?}", typ),
    }
}
