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

#![feature(proc_macro)]

extern crate tql;
#[macro_use]
extern crate tql_macros;

#[macro_use]
mod connection;

backend_extern_crate!();

use tql::{ForeignKey, PrimaryKey};
use tql_macros::to_sql;

#[derive(SqlTable)]
#[allow(dead_code)]
struct Table {
    id: PrimaryKey,
    field1: String,
    field2: i32,
    field3: i32,
    related_field: ForeignKey<RelatedTable>,
}

#[derive(SqlTable)]
#[allow(dead_code)]
struct RelatedTable {
    id: PrimaryKey,
    field1: String,
}

#[test]
fn test_update() {
    assert_eq!(
        "UPDATE Table SET field1 = 'value1', field2 = 55 WHERE Table.id = 1",
        to_sql!(Table.get(1).update(field1 = "value1", field2 = 55))
    );
    assert_eq!(
        "UPDATE Table SET field1 = 'value1', field2 = $1 WHERE Table.id = 1",
        to_sql!(Table.filter(id == 1).update(field1 = "value1", field2 = new_field2))
    );
}

#[test]
fn test_update_operation() {
    assert_eq!(
        "UPDATE Table SET field2 = field2 + 1 WHERE Table.id = 1",
        to_sql!(Table.get(1).update(field2 += 1))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 - 3 WHERE Table.id = 1",
        to_sql!(Table.get(1).update(field2 -= 3))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 % 7 WHERE Table.id = 1",
        to_sql!(Table.get(1).update(field2 %= 7))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 * 2 WHERE Table.id = 1",
        to_sql!(Table.get(1).update(field2 *= 2))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 / 3 WHERE Table.id = 1",
        to_sql!(Table.get(1).update(field2 /= 3))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 + 10, field3 = field3 / 3 WHERE Table.id = 1",
        to_sql!(Table.get(1).update(field2 += 10, field3 /= 3))
    );
}
