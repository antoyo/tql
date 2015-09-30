use syntax::ast::Expr_::ExprLit;
use syntax::ast::Lit_::LitInt;

use ast::{Expression, Fields, Filter, Filters, FilterExpression, Identifier, LogicalOperator, Order, RelationalOperator, Query};

pub trait ToSql {
    fn to_sql(&self) -> String;
}

impl<'a> ToSql for Fields<'a> {
    fn to_sql(&self) -> String {
        match *self {
            Fields::All => "*".to_string(),
            _ => unimplemented!(),
        }
    }
}

impl ToSql for Filter {
    fn to_sql(&self) -> String {
        self.operand1.to_sql() + " " + &self.operator.to_sql() + " " + &self.operand2.to_sql()
    }
}

impl ToSql for FilterExpression {
    fn to_sql(&self) -> String {
        match *self {
            FilterExpression::Filter(ref filter) => format!("{}", filter.to_sql()),
            FilterExpression::Filters(ref filters) => format!("{}", filters.to_sql()),
            FilterExpression::NoFilters => "".to_string(),
        }
    }
}

impl ToSql for Filters {
    fn to_sql(&self) -> String {
        self.operand1.to_sql() + " " + &self.operator.to_sql() + " " + &self.operand2.to_sql()
    }
}

impl ToSql for LogicalOperator {
    fn to_sql(&self) -> String {
        match *self {
            LogicalOperator::And => "AND".to_string(),
            LogicalOperator::Not => "NOT".to_string(),
            LogicalOperator::Or => "OR".to_string(),
        }
    }
}

impl ToSql for Order {
    fn to_sql(&self) -> String {
        match *self {
            Order::Ascending(ref field) => field.clone(),
            Order::Descending(ref field) => field.clone() + " DESC",
        }
    }
}

impl ToSql for [Order] {
    fn to_sql(&self) -> String {
        if self.len() > 0 {
            " ORDER BY ".to_string() + &self.iter().map(ToSql::to_sql).collect::<Vec<_>>().join(", ")
        }
        else {
            "".to_string()
        }
    }
}

impl ToSql for RelationalOperator {
    fn to_sql(&self) -> String {
        match *self {
            RelationalOperator::Equal => "=".to_string(),
            RelationalOperator::LesserThan => "<".to_string(),
            RelationalOperator::LesserThanEqual => "<=".to_string(),
            RelationalOperator::NotEqual => "<>".to_string(),
            RelationalOperator::GreaterThan => ">=".to_string(),
            RelationalOperator::GreaterThanEqual => ">".to_string(),
        }
    }
}

impl<'a> ToSql for Query<'a> {
    fn to_sql(&self) -> String {
        match *self {
            Query::CreateTable { .. } => "".to_string(), // TODO
            Query::Delete { .. } => "".to_string(), // TODO
            Query::Insert { .. } => "".to_string(), // TODO
            Query::Select{ref fields, ref filter, joins, ref limit, order, ref table} => {
                let where_clause = match filter {
                    &FilterExpression::Filter(_) => " WHERE ",
                    &FilterExpression::Filters(_) => " WHERE ",
                    &FilterExpression::NoFilters => "",
                };
                format!("SELECT {} FROM {}{}{}{}", fields.to_sql(), table, where_clause, filter.to_sql(), order.to_sql())
            },
            Query::Update { .. } => "".to_string(), // TODO
        }
    }
}

impl ToSql for Identifier {
    fn to_sql(&self) -> String {
        self.clone()
    }
}

impl ToSql for Expression {
    fn to_sql(&self) -> String {
        match self.node {
            ExprLit(ref literal) => {
                match literal.node {
                    LitInt(number, _) => number.to_string(),
                    _ => "?".to_string(), // TODO
                }
            },
            _ => "?".to_string(),
        }
    }
}
