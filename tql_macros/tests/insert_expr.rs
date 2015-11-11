#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use postgres::{Connection, SslMode};
use tql::PrimaryKey;

#[SqlTable]
#[allow(dead_code)]
#[derive(Debug)]
struct SqlTable {
    primary_key: PrimaryKey,
    field1: String,
    field2: i32,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", &SslMode::None).unwrap()
}

#[test]
fn test_insert() {
    let connection = get_connection();

    let _ = sql!(SqlTable.drop()); // NOTE: In case a test failed.
    let _ = sql!(SqlTable.create());

    let id = sql!(SqlTable.insert(field1 = "value1", field2 = 55)).unwrap();
    assert_eq!(1, id);

    let table = sql!(SqlTable.get(id)).unwrap();
    assert_eq!("value1", table.field1);
    assert_eq!(55, table.field2);

    let new_field2 = 42;
    let id = sql!(SqlTable.insert(field1 = "value2", field2 = new_field2)).unwrap();
    assert_eq!(2, id);

    let table = sql!(SqlTable.get(id)).unwrap();
    assert_eq!("value2", table.field1);
    assert_eq!(42, table.field2);

    let _ = sql!(SqlTable.drop());
}
