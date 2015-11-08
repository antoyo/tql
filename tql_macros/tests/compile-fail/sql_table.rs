//! Tests of the `#[SqlTable]` attribute.

#![feature(plugin)]
#![plugin(tql_macros)]

struct Connection {
    value: String,
}

#[SqlTable]
struct Table<'a> {
    string: &'a str,
    //~^ ERROR use of unsupported type name `&'a str` [E0412]
    //~| run `rustc --explain E0412` to see a detailed explanation
    connection: Connection,
    //~^ ERROR use of unsupported type name `Connection` [E0412]
    connection2: Option<Connection>,
    //~^ ERROR use of unsupported type name `Connection` [E0412]
    nested_options: Option<Option<String>>,
    //~^ ERROR use of unsupported type name `Option<String>` [E0412]
}
