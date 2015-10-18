//! The PostgreSQL code generator.

use std::str::from_utf8;

use syntax::ast::Expr_::ExprLit;
use syntax::ast::Lit_::{LitBool, LitByte, LitByteStr, LitChar, LitFloat, LitFloatUnsuffixed, LitInt, LitStr};

use ast::{Assignment, Expression, FieldList, Filter, Filters, FilterExpression, Identifier, Join, Limit, LogicalOperator, Order, RelationalOperator, Query};
use ast::Limit::{EndRange, Index, LimitOffset, NoLimit, Range, StartRange};
use sql::escape;

pub trait ToSql {
    fn to_sql(&self) -> String;
}

impl ToSql for Assignment {
    fn to_sql(&self) -> String {
        self.identifier.to_sql() + " = " + &self.value.to_sql()
    }
}

impl ToSql for [Assignment] {
    fn to_sql(&self) -> String {
        self.into_iter().map(ToSql::to_sql).collect::<Vec<_>>().join(", ")
    }
}

impl ToSql for Expression {
    fn to_sql(&self) -> String {
        match self.node {
            ExprLit(ref literal) => {
                match literal.node {
                    // TODO: ne pas utiliser unwrap().
                    LitBool(boolean) => boolean.to_string().to_uppercase(),
                    LitByte(byte) => "'".to_owned() + &escape((byte as char).to_string()) + "'",
                    LitByteStr(ref bytestring) => "'".to_owned() + &escape(from_utf8(&bytestring[..]).unwrap().to_owned()) + "'",
                    LitChar(character) => "'".to_owned() + &escape(character.to_string()) + "'",
                    LitFloat(ref float, _) => float.to_string(),
                    LitFloatUnsuffixed(ref float) => float.to_string(),
                    LitInt(number, _) => number.to_string(),
                    LitStr(ref string, _) => "'".to_owned() + &escape(string.to_string()) + "'",
                }
            },
            _ => "?".to_owned(),
        }
    }
}

impl ToSql for [Expression] {
    fn to_sql(&self) -> String {
        self.iter().map(ToSql::to_sql).collect::<Vec<_>>().join(", ")
    }
}

impl ToSql for FieldList {
    fn to_sql(&self) -> String {
        self.join(", ")
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
            FilterExpression::Filter(ref filter) => filter.to_sql(),
            FilterExpression::Filters(ref filters) => filters.to_sql(),
            FilterExpression::NegFilter(ref filter) => "NOT ".to_owned() + &filter.to_sql(),
            FilterExpression::NoFilters => "".to_owned(),
            FilterExpression::ParenFilter(ref filter) => "(".to_owned() + &filter.to_sql() + ")"
        }
    }
}

impl ToSql for Filters {
    fn to_sql(&self) -> String {
        self.operand1.to_sql() + " " + &self.operator.to_sql() + " " + &self.operand2.to_sql()
    }
}

impl ToSql for Join {
    fn to_sql(&self) -> String {
        " INNER JOIN ".to_owned() + &self.right_table + " ON " + &self.left_table + "." + &self.left_field + "_id = " + &self.right_table + "." + &self.right_field
    }
}

impl ToSql for [Join] {
    fn to_sql(&self) -> String {
        if self.len() > 0 {
            self.iter().map(ToSql::to_sql).collect::<Vec<_>>().join(" ")
        }
        else {
            "".to_owned()
        }
    }
}

impl ToSql for Identifier {
    fn to_sql(&self) -> String {
        self.clone()
    }
}

impl ToSql for Limit {
    fn to_sql(&self) -> String {
        match *self {
            EndRange(ref expression) => " LIMIT ".to_owned() + &expression.to_sql(),
            Index(ref expression) => " OFFSET ".to_owned() + &expression.to_sql() + " LIMIT 1",
            LimitOffset(ref expression1, ref expression2) => " OFFSET ".to_owned() + &expression2.to_sql() + " LIMIT " + &expression1.to_sql(),
            NoLimit => "".to_owned(),
            Range(ref expression1, ref expression2) => " OFFSET ".to_owned() + &expression1.to_sql() + " LIMIT " + &expression2.to_sql(),
            StartRange(ref expression) => " OFFSET ".to_owned() + &expression.to_sql(),
        }
    }
}

impl ToSql for LogicalOperator {
    fn to_sql(&self) -> String {
        match *self {
            LogicalOperator::And => "AND".to_owned(),
            LogicalOperator::Not => "NOT".to_owned(),
            LogicalOperator::Or => "OR".to_owned(),
        }
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

impl ToSql for [Order] {
    fn to_sql(&self) -> String {
        if self.len() > 0 {
            " ORDER BY ".to_owned() + &self.iter().map(ToSql::to_sql).collect::<Vec<_>>().join(", ")
        }
        else {
            "".to_owned()
        }
    }
}

impl ToSql for Query {
    fn to_sql(&self) -> String {
        match *self {
            Query::CreateTable { .. } => "".to_owned(), // TODO
            Query::Delete { .. } => "".to_owned(), // TODO
            Query::Insert { ref assignments, ref table } => {
                let fields: Vec<_> = assignments.iter().map(|assign| assign.identifier.clone()).collect();
                let values: Vec<_> = assignments.iter().map(|assign| assign.value.clone()).collect();
                replace_placeholder(format!("INSERT INTO {}({}) VALUES({})", table, fields.to_sql(), values.to_sql()))
            },
            Query::Select{ref fields, ref filter, ref joins, ref limit, ref order, ref table} => {
                let where_clause = filter_to_where_clause(filter);
                replace_placeholder(format!("SELECT {} FROM {}{}{}{}{}{}", fields.to_sql(), table, joins.to_sql(), where_clause, filter.to_sql(), order.to_sql(), limit.to_sql()))
            },
            Query::Update { ref assignments, ref filter, ref table } => {
                let where_clause = filter_to_where_clause(filter);
                replace_placeholder(format!("UPDATE {} SET {}{}{}", table, assignments.to_sql(), where_clause, filter.to_sql()))
            },
        }
    }
}

impl ToSql for RelationalOperator {
    fn to_sql(&self) -> String {
        match *self {
            RelationalOperator::Equal => "=".to_owned(),
            RelationalOperator::LesserThan => "<".to_owned(),
            RelationalOperator::LesserThanEqual => "<=".to_owned(),
            RelationalOperator::NotEqual => "<>".to_owned(),
            RelationalOperator::GreaterThan => ">=".to_owned(),
            RelationalOperator::GreaterThanEqual => ">".to_owned(),
        }
    }
}

fn filter_to_where_clause(filter: &FilterExpression) -> &str {
    match *filter {
        FilterExpression::Filter(_) | FilterExpression::Filters(_) | FilterExpression::NegFilter(_) | FilterExpression::ParenFilter(_) => " WHERE ",
        FilterExpression::NoFilters => "",
    }
}

// TODO: essayer de trouver une meilleure façon de mettre les symboles ($1, $2, …) dans la requête.
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
