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

#![feature(proc_macro)]

extern crate rusqlite;
extern crate tql;
#[macro_use]
extern crate tql_macros;

use tql::{ForeignKey, PrimaryKey};
use tql_macros::to_sql;

#[derive(SqlTable)]
#[allow(dead_code)]
struct Table {
    id: PrimaryKey,
    field1: String,
    field2: i32,
    related_field: ForeignKey<RelatedTable>,
    optional_field: Option<i32>,
}

#[derive(SqlTable)]
#[allow(dead_code)]
struct RelatedTable {
    id: PrimaryKey,
    field1: String,
}

#[test]
fn test_insert() {
    assert_eq!(
        "INSERT INTO RelatedTable(field1) VALUES('test')",
        to_sql!(RelatedTable.insert(field1 = "test"))
    );
    assert_eq!(
        "INSERT INTO Table(field1, field2, related_field) VALUES('value1', 55, $1)",
        to_sql!(Table.insert(field1 = "value1", field2 = 55, related_field = related_object))
    );
    assert_eq!(
        "INSERT INTO Table(field1, field2, related_field) VALUES('value1', $1, $2)",
        to_sql!(Table.insert(field1 = "value1", field2 = new_field2, related_field = related_object))
    );
    assert_eq!(
        "INSERT INTO Table(field1, field2, related_field, optional_field) VALUES('value1', 55, $1, 42)",
        to_sql!(Table.insert(field1 = "value1", field2 = 55, related_field = related_object, optional_field = 42))
    );
}
