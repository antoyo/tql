use ast::{Expression, Fields, Filter, Filters, FilterExpression, Identifier, LogicalOperator, RelationalOperator, Query};

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
            FilterExpression::Filter(ref filter) => filter.to_sql(),
            FilterExpression::Filters(ref filters) => filters.to_sql(),
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
            Query::Select{ref fields, ref filter, ref joins, ref limit, ref order, ref table} => {
                let fields_sql = fields.to_sql();
                match filter {
                    &FilterExpression::Filters(ref filters) => format!("SELECT {} FROM {} WHERE {}", fields_sql, table, filters.to_sql()),
                    &FilterExpression::Filter(ref filter) => format!("SELECT {} FROM {} WHERE {}", fields_sql, table, filter.to_sql()),
                    &FilterExpression::NoFilters => format!("SELECT {} FROM {}", fields_sql, table),
                }
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
        "?".to_string()
    }
}
