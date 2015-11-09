//! Tests of the type analyzer lint for the `#[SqlTable]` attribute.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use tql::{ForeignKey, PrimaryKey};

struct Connection {
    value: String,
}

#[SqlTable]
struct Table {
    //~^ WARNING No primary key found
    field1: String,
    related_field1: ForeignKey<Connection>,
    //~^ ERROR `Connection` does not name an SQL table [E0422]
    //~| HELP run `rustc --explain E0422` to see a detailed explanation
    //~| HELP did you forget to add the #[sql_table] attribute on the Connection struct?
    related_field2: ForeignKey<RelatedTable>,
}

#[SqlTable]
struct RelatedTable {
    id: PrimaryKey,
}
