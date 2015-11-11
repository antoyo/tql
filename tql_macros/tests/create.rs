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
fn test_create() {
    assert_eq!(
        "CREATE TABLE Table (field1 CHARACTER VARYING NOT NULL, field2 INTEGER NOT NULL, field3 INTEGER, id SERIAL PRIMARY KEY NOT NULL, related_field INTEGER REFERENCES RelatedTable(id) NOT NULL)",
        to_sql!(Table.create())
    );
    assert_eq!(
        "CREATE TABLE RelatedTable (field1 CHARACTER VARYING NOT NULL, id SERIAL PRIMARY KEY NOT NULL)",
        to_sql!(RelatedTable.create())
    );
}
