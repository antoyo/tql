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

extern crate postgres;
extern crate tql;
#[macro_use]
extern crate tql_macros;

use postgres::{Connection, TlsMode};
use tql::PrimaryKey;
use tql_macros::sql;

mod teardown;

use teardown::TearDown;

#[derive(SqlTable)]
#[allow(dead_code)]
struct TableAggregateExpr {
    primary_key: PrimaryKey,
    field1: String,
    field2: i32,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", TlsMode::None).unwrap()
}

#[test]
fn test_aggregate() {
    let connection = get_connection();

    let _teardown = TearDown::new(|| {
        let _ = sql!(TableAggregateExpr.drop());
    });

    let _ = sql!(TableAggregateExpr.create());

    sql!(TableAggregateExpr.insert(field1 = "test", field2 = 55)).unwrap();
    sql!(TableAggregateExpr.insert(field1 = "testing", field2 = 12)).unwrap();

    let new_field1 = 42;
    sql!(TableAggregateExpr.insert(field1 = "test", field2 = new_field1)).unwrap();

    let aggregate = sql!(TableAggregateExpr.aggregate(avg(field2))).unwrap();
    assert_eq!(36, aggregate.field2_avg); // NOTE: round((55 + 12 + 42) / 3) = 36.

    let mut aggregates = sql!(TableAggregateExpr
          .values(field1)
          .aggregate(avg(field2)));
    assert_eq!(2, aggregates.len());
    aggregates.sort_by_key(|agg| agg.field2_avg);
    assert_eq!(12, aggregates[0].field2_avg); // NOTE: round(12 / 1) = 12.
    assert_eq!(49, aggregates[1].field2_avg); // NOTE: round((55 + 42) / 3) = 49.

    let aggregate = sql!(TableAggregateExpr.aggregate(average = avg(field2))).unwrap();
    assert_eq!(36, aggregate.average); // NOTE: round((55 + 12 + 42) / 3) = 36.

    let aggregates = sql!(TableAggregateExpr.values(field1).aggregate(average = avg(field2)).filter(average < 20));
    assert_eq!(1, aggregates.len());
    assert_eq!(12, aggregates[0].average); // NOTE: round(12 / 1) = 12.

    let aggregates = sql!(TableAggregateExpr.values(field1).aggregate(avg(field2)).filter(field2_avg < 20));
    assert_eq!(1, aggregates.len());
    assert_eq!(12, aggregates[0].field2_avg); // NOTE: round(12 / 1) = 12.

    let aggregates = sql!(TableAggregateExpr
        .filter(field2 > 10)
        .values(field1)
        .aggregate(avg(field2)).filter(field2_avg < 20));
    assert_eq!(1, aggregates.len());
    assert_eq!(12, aggregates[0].field2_avg); // NOTE: round(12 / 1) = 12.

    let aggregates = sql!(TableAggregateExpr.filter(field2 > 10).values(field1).aggregate(average = avg(field2)).filter(average < 20));
    assert_eq!(1, aggregates.len());
    assert_eq!(12, aggregates[0].average); // NOTE: round(12 / 1) = 12.

    let value1 = 10;
    let aggregates = sql!(TableAggregateExpr
        .filter(field2 > value1)
        .values(field1)
        .aggregate(average = avg(field2))
        .filter(average < 20));
    assert_eq!(1, aggregates.len());
    assert_eq!(12, aggregates[0].average); // NOTE: round(12 / 1) = 12.

    let value2 = 20;
    let aggregates = sql!(TableAggregateExpr
        .filter(field2 > value1)
        .values(field1)
        .aggregate(average = avg(field2))
        .filter(average < value2));
    assert_eq!(1, aggregates.len());
    assert_eq!(12, aggregates[0].average); // NOTE: round(12 / 1) = 12.
}
