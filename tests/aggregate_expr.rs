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

use tql::PrimaryKey;
use tql_macros::sql;

#[macro_use]
mod connection;
mod teardown;

backend_extern_crate!();

use connection::get_connection;
use teardown::TearDown;

#[derive(SqlTable)]
#[allow(dead_code)]
struct TableAggregateExpr {
    primary_key: PrimaryKey,
    field1: String,
    field2: i32,
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
    assert_eq!((55.0 + 12.0 + 42.0) / 3.0, aggregate.field2_avg);

    let mut aggregates = sql!(TableAggregateExpr
          .values(field1)
          .aggregate(avg(field2)));
    assert_eq!(2, aggregates.len());
    aggregates.sort_by(|x, y| x.field2_avg.partial_cmp(&y.field2_avg).expect("aggregate value"));
    assert_eq!(12.0, aggregates[0].field2_avg); // NOTE: round(12 / 1) = 12.
    assert_eq!(48.5, aggregates[1].field2_avg); // NOTE: round((55 + 42) / 3) = 49.

    let aggregate = sql!(TableAggregateExpr.aggregate(average = avg(field2))).unwrap();
    assert_eq!((55.0 + 12.0 + 42.0) / 3.0, aggregate.average);

    let aggregates = sql!(TableAggregateExpr.values(field1).aggregate(average = avg(field2)).filter(average < 20.0));
    assert_eq!(1, aggregates.len());
    assert_eq!(12.0, aggregates[0].average); // NOTE: round(12 / 1) = 12.

    let aggregates = sql!(TableAggregateExpr.values(field1).aggregate(avg(field2)).filter(field2_avg < 20.0));
    assert_eq!(1, aggregates.len());
    assert_eq!(12.0, aggregates[0].field2_avg); // NOTE: round(12 / 1) = 12.

    let aggregates = sql!(TableAggregateExpr
        .filter(field2 > 10)
        .values(field1)
        .aggregate(avg(field2)).filter(field2_avg < 20.0));
    assert_eq!(1, aggregates.len());
    assert_eq!(12.0, aggregates[0].field2_avg); // NOTE: round(12 / 1) = 12.

    let aggregates = sql!(TableAggregateExpr.filter(field2 > 10).values(field1)
                          .aggregate(average = avg(field2)).filter(average < 20.0));
    assert_eq!(1, aggregates.len());
    assert_eq!(12.0, aggregates[0].average); // NOTE: round(12 / 1) = 12.

    let value1 = 10;
    let aggregates = sql!(TableAggregateExpr
        .filter(field2 > value1)
        .values(field1)
        .aggregate(average = avg(field2))
        .filter(average < 20.0));
    assert_eq!(1, aggregates.len());
    assert_eq!(12.0, aggregates[0].average); // NOTE: round(12 / 1) = 12.

    let value2 = 20.0;
    let aggregates = sql!(TableAggregateExpr
        .filter(field2 > value1)
        .values(field1)
        .aggregate(average = avg(field2))
        .filter(average < value2));
    assert_eq!(1, aggregates.len());
    assert_eq!(12.0, aggregates[0].average); // NOTE: round(12 / 1) = 12.
}
