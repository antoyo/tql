#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use tql::{ForeignKey, PrimaryKey};

#[SqlTable]
#[derive(Debug)]
struct Table {
    id: PrimaryKey,
    field1: String,
    field2: i32,
    field3: Option<i32>,
    related_field: ForeignKey<RelatedTable>,
}

#[SqlTable]
#[derive(Debug)]
struct RelatedTable {
    id: PrimaryKey,
    field1: String,
}

#[test]
fn test_drop() {
    assert_eq!(
        "DROP TABLE Table",
        to_sql!(Table.drop())
    );
    assert_eq!(
        "DROP TABLE RelatedTable",
        to_sql!(RelatedTable.drop())
    );
}
