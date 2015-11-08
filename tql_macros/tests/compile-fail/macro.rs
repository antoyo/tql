//! Tests of the macro.

#![feature(plugin)]
#![plugin(tql_macros)]

struct Connection {
    value: String,
}

#[SqlTable]
struct Table {
    field1: String,
    i32_field: i32,
}

fn main() {
    to_sql!();
    //~^ ERROR this macro takes 1 parameter but 0 parameters were supplied [E0061]

    sql!();
    //~^ ERROR this macro takes 1 parameter but 0 parameters were supplied [E0061]

    sql!(Table);
    //~^ ERROR `Table` is the name of a struct, but this expression uses it like a method name [E0423]
    //~| HELP run `rustc --explain E0423` to see a detailed explanation
    //~| HELP did you mean to write `Table.method()`?

    sql!(Table());
    //~^ ERROR Expected method call
}
