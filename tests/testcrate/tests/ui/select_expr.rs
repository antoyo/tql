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

//! Tests of the type analyzer lint for a `Query::Select`.

#![feature(proc_macro)]

extern crate tql;
#[macro_use]
extern crate tql_macros;

#[macro_use]
mod connection;
backend_extern_crate!();

use tql::{ForeignKey, PrimaryKey};
use tql_macros::sql;

use connection::{Connection, get_connection};

#[derive(SqlTable)]
struct OtherTable {
    id: PrimaryKey,
    field1: i32,
    field2: String,
}

#[derive(SqlTable)]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
    other: ForeignKey<OtherTable>,
}

fn main() {
    let connection = get_connection();

    let index = 24i32;
    sql!(Table[index]);
    // ~^ ERROR mismatched types:
    // ~| expected `i64`
    // ~| found `i32` [E0308]

    sql!(Table.filter(i32_field == 42)[index]);
    // ~^ ERROR mismatched types:
    // ~| expected `i64`
    // ~| found `i32` [E0308]

    let value = 20;
    let value1 = 42;
    sql!(Table.filter(i32_field > value && field1 == value1));
    //~^ ERROR mismatched types
    //~| NOTE expected &str, found integral variable
    //~| NOTE expected type `&str`
    //~| found type `{integer}`

    let value = 20i64;
    sql!(Table.filter(i32_field > value));
    //~^ ERROR mismatched types
    //~| NOTE expected i64, found i32
    // FIXME: When this issue (https://github.com/rust-lang/rust/issues/46609) is fixed, use the following:
    // ~| NOTE expected i32, found struct i64
    // ~| NOTE expected `i32`
    // ~| found `i64`

    let table1 = sql!(Table.get(1)).unwrap();
    sql!(Table.filter(other == table1));
    //~^ ERROR mismatched types
    //~| NOTE expected struct `Table`, found struct `OtherTable`
    //~| NOTE expected type `Table`
    //~| found type `OtherTable`
    // FIXME:
    // ~| NOTE expected struct `OtherTable`, found struct `Table`
    // ~| NOTE expected `OtherTable`
    // ~| found `Table`

    let other = sql!(OtherTable.get(1)).unwrap();
    sql!(Table.filter(other == other));
}
