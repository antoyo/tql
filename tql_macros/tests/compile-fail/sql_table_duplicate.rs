//! Tests of the syntax extension errors.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate tql;

#[SqlTable]
struct Table {
    field1: String,
}

#[SqlTable]
struct Table {
    //~^ ERROR duplicate definition of table `Table` [E0428]
    //~| HELP run `rustc --explain E0428` to see a detailed explanation
}
