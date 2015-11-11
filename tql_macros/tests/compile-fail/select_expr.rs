//! Tests of the type analyzer lint for a `Query::Select`.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use postgres::{Connection, SslMode};
use tql::{ForeignKey, PrimaryKey};

#[SqlTable]
#[derive(Debug)]
struct OtherTable {
    id: PrimaryKey,
    field1: i32,
    field2: String,
}

#[SqlTable]
#[derive(Debug)]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
    other: ForeignKey<OtherTable>,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", &SslMode::None).unwrap()
}

fn main() {
    let connection = get_connection();

    let index = 24;
    sql!(Table[index]);
    //~^ ERROR mismatched types:
    //~| expected `i64`,
    //~| found `i32` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(i32_field == 42)[index]);
    //~^ ERROR mismatched types:
    //~| expected `i64`,
    //~| found `i32` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    let value = 20;
    let value1 = 42;
    sql!(Table.filter(i32_field > value && field1 == value1));
    //~^ ERROR mismatched types:
    //~| expected `String`,
    //~| found `i32` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    let value = 20i64;
    sql!(Table.filter(i32_field > value));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `i64` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    let table1 = sql!(Table.get(1)).unwrap();
    sql!(Table.filter(other == table1));
    //~^ ERROR mismatched types:
    //~| expected `OtherTable`,
    //~| found `Table` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    let other = sql!(OtherTable.get(1)).unwrap();
    sql!(Table.filter(other == other));
}
