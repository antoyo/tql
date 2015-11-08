//! Tests of the delete() method.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate tql;

use tql::PrimaryKey;

#[SqlTable]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
}

fn main() {
    sql!(Table.filter(field1 == 42).delete());
    //~^ ERROR mismatched types:
    //~| expected `String`,
    //~| found `integral variable` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    let _ = sql!(Table.filter(id == 1).delete(id == 1));
    //~^ ERROR this method takes 0 parameters but 1 parameter was supplied [E0061]
    //~| HELP run `rustc --explain E0061` to see a detailed explanation

    sql!(Table.delete());
    //~^ WARNING delete() without filters
}
