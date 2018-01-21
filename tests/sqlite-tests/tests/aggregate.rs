/*
 * Copyright (c) 2018 Boucher, Antoni <bouanto@zoho.com>
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

use tql::PrimaryKey;
use tql_macros::to_sql;

#[derive(SqlTable)]
#[allow(dead_code)]
struct Table {
    id: PrimaryKey,
    field1: String,
    field2: i32,
}

#[test]
fn test_aggregate() {
    assert_eq!(
        "SELECT AVG(field2) FROM Table",
        to_sql!(Table.aggregate(avg(field2)))
    );
    assert_eq!(
        "SELECT AVG(field2) FROM Table GROUP BY field1",
        to_sql!(Table.values(field1).aggregate(avg(field2)))
    );
    assert_eq!(
        "SELECT AVG(field2) FROM Table",
        to_sql!(Table.aggregate(average = avg(field2)))
    );
    assert_eq!(
        "SELECT AVG(field2) FROM Table GROUP BY field1 HAVING AVG(field2) < 20",
        to_sql!(Table.values(field1).aggregate(average = avg(field2)).filter(average < 20))
    );
    assert_eq!(
        "SELECT AVG(field2) FROM Table GROUP BY field1 HAVING AVG(field2) < 20",
        to_sql!(Table.values(field1).aggregate(avg(field2)).filter(field2_avg < 20))
    );
    assert_eq!(
        "SELECT AVG(field2) FROM Table WHERE Table.field2 > 10 GROUP BY field1 HAVING AVG(field2) < 20",
        to_sql!(Table.filter(field2 > 10).values(field1).aggregate(avg(field2)).filter(field2_avg < 20))
    );
    assert_eq!(
        "SELECT AVG(field2) FROM Table WHERE Table.field2 > 10 GROUP BY field1 HAVING AVG(field2) < 20",
        to_sql!(Table.filter(field2 > 10).values(field1).aggregate(average = avg(field2)).filter(average < 20))
    );
}
