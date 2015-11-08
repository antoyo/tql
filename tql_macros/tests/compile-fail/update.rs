//! Tests of the update() method.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate tql;

use tql::PrimaryKey;

#[SqlTable]
#[derive(Debug)]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
}

fn main() {
    let value = 42;
    let _ = sql!(Table.filter(id == 1).update(field1 = 42, i32_field = value));
    //~^ ERROR mismatched types:
    //~| expected `String`,
    //~| found `integral variable` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)
}
