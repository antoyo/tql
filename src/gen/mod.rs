use ast::{Expression, Filter, Identifier, Operator, Query};

pub trait ToSql {
    fn to_sql(&self) -> String;
}

impl ToSql for Filter {
    fn to_sql(&self) -> String {
        self.operand1.to_sql() + " " + &self.operator.to_sql() + " " + &self.operand2.to_sql()
    }
}

impl ToSql for Operator {
    fn to_sql(&self) -> String {
        match *self {
            Operator::And => "AND".to_string(),
            Operator::Or => "OR".to_string(),
            Operator::Equal => "=".to_string(),
            Operator::LesserThan => "<".to_string(),
            Operator::LesserThanEqual => "<=".to_string(),
            Operator::NotEqual => "<>".to_string(),
            Operator::GreaterThan => ">=".to_string(),
            Operator::GreaterThanEqual => ">".to_string(),
        }
    }
}

impl ToSql for Query {
    fn to_sql(&self) -> String {
        match *self {
            Query::CreateTable => "".to_string(),
            Query::Delete => "".to_string(),
            Query::Insert => "".to_string(),
            Query::Select{ref filter, ref table} => {
                match filter {
                    &Some(ref filter) => format!("SELECT * FROM {} {}", table, filter.to_sql()),
                    &None => format!("SELECT * FROM {}", table),
                }
            },
            Query::Update => "".to_string(),
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
        "?".to_string()
    }
}
