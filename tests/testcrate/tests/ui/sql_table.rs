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

//! Tests of the `#[SqlTable]` attribute.

#![feature(proc_macro)] // FIXME: bad span for field nested_options in stderr (should be on Option<String>, not just Option).

#[macro_use]
extern crate tql_macros;

struct Connection {
    value: String,
}

#[derive(SqlTable)]
struct Table<'a> {
    //~^ WARNING No primary key found
    string: &'a str,
    //~^ ERROR use of unsupported type name `& 'a str`
    connection: Connection,
    //~^ ERROR use of unsupported type name `Connection`
    connection2: Option<Connection>,
    //~^ ERROR use of unsupported type name `Connection`
    nested_options: Option<Option<String>>,
    //~^ ERROR use of unsupported type name `Option<String>`
    datetime: DateTime,
    //~^ ERROR use of unsupported type name `DateTime`
    datetime_i32: DateTime<i32>,
    //~^ ERROR use of unsupported type name `DateTime<i32>`
    foreign_value: ForeignKey,
    //~^ ERROR use of unsupported type name `ForeignKey`
    optional_value: Option,
    //~^ ERROR use of unsupported type name `Option`
    vector: Vec,
    //~^ ERROR use of unsupported type name `Vec`
    vector_i32: Vec<i32>,
    //~^ ERROR use of unsupported type name `Vec<i32>`
}
