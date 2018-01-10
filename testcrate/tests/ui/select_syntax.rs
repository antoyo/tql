/*
 * Copyright (c) 2017-2018 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Tests of the methods related to `Query::Select`.

#![feature(proc_macro)]

extern crate postgres;
extern crate tql;
#[macro_use]
extern crate tql_macros;

use postgres::{Connection, TlsMode};
use tql::PrimaryKey;
use tql_macros::sql;

#[derive(SqlTable)]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", TlsMode::None).unwrap()
}

fn main() {
    let connection = get_connection();

    sql!(Table.all()[.."auinesta"]);
    // ~^ ERROR mismatched types:
    // ~| expected `i64`,
    // ~| found `String`
    // ~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.all()[true..false]);
    // ~^ ERROR mismatched types:
    // ~| expected `i64`,
    // ~| found `bool`
    // ~| NOTE in this expansion of sql! (defined in tql)
    // ~| ERROR mismatched types:
    // ~| expected `i64`,
    // ~| found `bool`
    // ~| NOTE in this expansion of sql! (defined in tql)
    // FIXME: the position should be on the star for the next sql!() query.

    sql!(Table.filter(i32_field < 100 && field1 == "value1").sort(*i32_field, *field1));
    //~^ ERROR Expected - or identifier
    //~| ERROR Expected - or identifier

    sql!(TestTable.flter(field1 == "value"));
    //~| ERROR no method named `flter` found in tql
    //~| HELP did you mean filter?

    sql!(Table.all(id == 1));
    //~^ ERROR this method takes 0 parameters but 1 parameter was supplied
}
