#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use tql::PrimaryKey;

#[SqlTable]
#[allow(dead_code)]
#[derive(Debug)]
struct Table {
    id: PrimaryKey,
    field1: String,
}

#[test]
fn test_select() {
    assert_eq!("SELECT Table.field1, Table.id FROM Table", to_sql!(Table.all()));
}
