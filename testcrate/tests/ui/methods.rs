/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
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

//! Tests of the methods available in the filter() method.

#![feature(proc_macro)]

extern crate chrono;
extern crate postgres;
extern crate tql;
#[macro_use]
extern crate tql_macros;

use chrono::DateTime;
use chrono::offset::Utc;
use postgres::{Connection, TlsMode};
use tql::PrimaryKey;
use tql_macros::sql;

#[derive(SqlTable)]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
    date: DateTime<Utc>,
    option_field: Option<i32>,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", TlsMode::None).unwrap()
}

fn main() { // FIXME: bad span in stderr for line 59.
    let connection = get_connection();

    sql!(Table.filter(i32_field.year() == 2015));
    //~^ ERROR no method named `year` found for type `i32`

    sql!(Table.filter(date.test() == 2015));
    //~^ ERROR no method named `test` found for type `chrono::datetime::DateTime<chrono::offset::Utc>`

    sql!(Table.filter(date.yar() == 2015));
    //~^ ERROR no method named `yar` found for type `chrono::datetime::DateTime<chrono::offset::Utc>`
    //~| HELP did you mean year?

    sql!(Table.filter(dte.year() == 2015));
    //~^ ERROR attempted access of field `dte` on type `Table`, but no field with that name was found
    //~| HELP did you mean date?

    sql!(Table.filter(date.year()));
    //~^ ERROR mismatched types:
    //~| expected `bool`
    //~| found `i32`
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(field1.ends_with(1) == true));
    //~^ ERROR mismatched types:
    //~| expected `String`
    //~| found `integral variable`
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(field1.len() == "test"));
    //~^ ERROR mismatched types:
    //~| expected `i32`
    //~| found `String`
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(field1.len()));
    //~^ ERROR mismatched types:
    //~| expected `bool`
    //~| found `i32`
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(field1.len() && option_field.is_some()));
    //~^ ERROR mismatched types:
    //~| expected `bool`
    //~| found `i32`
    //~| NOTE in this expansion of sql! (defined in tql)
}
