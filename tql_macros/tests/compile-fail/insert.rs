//! Tests of the insert() method.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate tql;

use tql::PrimaryKey;

#[SqlTable]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
    field2: String,
}

fn main() {
    sql!(Table.insert(field1 = 42, i32_field = 91));
    //~^ ERROR missing fields: `field2` [E0063]
    //~| HELP run `rustc --explain E0063` to see a detailed explanation

    sql!(Table.insert(field1 = 42, i32_fild = 91));
    //~^ ERROR attempted access of field `i32_fild` on type `Table`, but no field with that name was found
    //~| HELP did you mean i32_field?

    sql!(Table.insert(i32_field += 42, field1 = "Test"));
    //~^ ERROR expected = but got +=
    //~| ERROR missing fields: `field2` [E0063]

    sql!(Table.insert(i32_field = 42, field1 -= "Test"));
    //~^ ERROR expected = but got -=
    //~| ERROR missing fields: `field2` [E0063]

    sql!(Table.insert(field1 = 42, i32_field = 91, field2 = "test"));
    //~^ ERROR mismatched types:
    //~| expected `String`,
    //~| found `integral variable` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)
}
