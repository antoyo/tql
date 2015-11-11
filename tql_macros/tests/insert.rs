#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use tql::{ForeignKey, PrimaryKey};

#[SqlTable]
#[allow(dead_code)]
#[derive(Debug)]
struct Table {
    id: PrimaryKey,
    field1: String,
    field2: i32,
    related_field: ForeignKey<RelatedTable>,
    optional_field: Option<i32>,
}

#[SqlTable]
#[allow(dead_code)]
#[derive(Debug)]
struct RelatedTable {
    id: PrimaryKey,
    field1: String,
}

#[test]
fn test_insert() {
    assert_eq!(
        "INSERT INTO RelatedTable(field1) VALUES('test') RETURNING id",
        to_sql!(RelatedTable.insert(field1 = "test"))
    );
    assert_eq!(
        "INSERT INTO Table(field1, field2, related_field) VALUES('value1', 55, $1) RETURNING id",
        to_sql!(Table.insert(field1 = "value1", field2 = 55, related_field = related_object))
    );
    assert_eq!(
        "INSERT INTO Table(field1, field2, related_field) VALUES('value1', $1, $2) RETURNING id",
        to_sql!(Table.insert(field1 = "value1", field2 = new_field2, related_field = related_object))
    );
    assert_eq!(
        "INSERT INTO Table(field1, field2, related_field, optional_field) VALUES('value1', 55, $1, 42) RETURNING id",
        to_sql!(Table.insert(field1 = "value1", field2 = 55, related_field = related_object, optional_field = 42))
    );
}
