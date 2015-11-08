//! Tests of the type analyzer lint for the `#[SqlTable]` attribute.

#![feature(plugin)]
#![plugin(tql_macros)]

struct Connection {
    value: String,
}

#[SqlTable]
struct Table<'a> {
    field1: String,
    //~^ WARNING No primary key found
    // TODO: this error should be on the previous line.
}
