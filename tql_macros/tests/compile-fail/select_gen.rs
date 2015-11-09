//! Tests of the generated code for a `Query::Select`.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use postgres::{Connection, SslMode};
use tql::PrimaryKey;

#[SqlTable]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", &SslMode::None).unwrap()
}

fn main() {
    let connection = get_connection();

    sql!(Table.filter(i32_field > value && field1 == value2));
    //~^ ERROR unresolved name `value` [E0425]
    //~| HELP run `rustc --explain E0425` to see a detailed explanation
    //~| ERROR unresolved name `value2` [E0425]
    //~| HELP run `rustc --explain E0425` to see a detailed explanation
}
