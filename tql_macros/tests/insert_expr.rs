#![feature(box_patterns, plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use postgres::{Connection, SslMode};
use postgres::error::Error::DbError;
use postgres::error::SqlState::UndefinedTable;
use tql::PrimaryKey;

#[SqlTable]
#[allow(dead_code)]
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

    let _ = sql!(SqlTable.drop());

    let result = sql!(SqlTable.insert(field1 = "value1", field2 = 55));
    match result {
        Err(DbError(box db_error)) => assert_eq!(UndefinedTable, *db_error.code()),
        Ok(_) => assert!(false),
        Err(_) => assert!(false),
    }

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
