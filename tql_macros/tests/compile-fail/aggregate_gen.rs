//! Tests of the type analyzer lint for a `Query::Aggregate`.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use postgres::{Connection, SslMode};
use tql::PrimaryKey;

#[SqlTable]
#[derive(Debug)]
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
    if let Some(aggregate) = sql!(Table.aggregate(average = avg(field2))) {
        println!("{}", aggregate.averag);
        //~^ ERROR attempted access of field `averag` on type `main::Aggregate`, but no field with that name was found
        //~| HELP did you mean `average`?
    }

    if let Some(aggregate) = sql!(Table.aggregate(average = avg(field2))) {
        println!("{}", aggregate.average);
    }
}
