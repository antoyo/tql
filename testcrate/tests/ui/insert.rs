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

//! Tests of the insert() method.

#![feature(proc_macro)]

extern crate tql;
#[macro_use]
extern crate tql_macros;

use tql::{ForeignKey, PrimaryKey};
use tql_macros::sql;

#[derive(SqlTable)]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
    field2: String,
    related_field: ForeignKey<RelatedTable>,
}

#[derive(SqlTable)]
struct RelatedTable {
    id: PrimaryKey,
}

fn main() {
    sql!(Table.insert(field1 = 42, i32_field = 91));
    //~^ ERROR missing fields: `field2`, `related_field`

    sql!(Table.insert(field1 = 42, i32_fild = 91));
    //~^ ERROR attempted access of field `i32_fild` on type `Table`, but no field with that name was found
    //~| HELP did you mean i32_field?

    sql!(Table.insert(i32_field += 42, field1 = "Test"));
    //~^ ERROR expected = but got +=
    //~| ERROR missing fields: `field2`, `related_field`

    sql!(Table.insert(i32_field = 42, field1 -= "Test"));
    //~^ ERROR expected = but got -=
    //~| ERROR missing fields: `field2`, `related_field`

    let related_field = RelatedTable {
        id: 1,
    };
    sql!(Table.insert(field1 = 42, i32_field = 91, field2 = "test", related_field = related_field));
    //~^ ERROR mismatched types:
    //~| expected `String`,
    //~| found `integral variable`
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.insert(field1 = "test", i32_field = 91, field2 = "test", related_field = 1));
    //~^ ERROR mismatched types:
    //~| expected `RelatedTable`,
    //~| found `integral variable`
    //~| NOTE in this expansion of sql! (defined in tql)
}
