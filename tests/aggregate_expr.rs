/*
 * Copyright (C) 2015  Boucher, Antoni <bouanto@zoho.com>
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#![feature(box_patterns, plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use postgres::{Connection, SslMode};
use tql::PrimaryKey;

mod teardown;

use teardown::TearDown;

#[SqlTable]
struct TableAggregateExpr {
    primary_key: PrimaryKey,
    field1: String,
    field2: i32,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", &SslMode::None).unwrap()
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

    let aggregates = sql!(TableAggregateExpr.values(field1).aggregate(avg(field2)));
    assert_eq!(2, aggregates.len());
    assert_eq!(49, aggregates[0].field2_avg); // NOTE: round((55 + 42) / 3) = 49.
    assert_eq!(12, aggregates[1].field2_avg); // NOTE: round(12 / 1) = 12.

    let aggregate = sql!(TableAggregateExpr.aggregate(average = avg(field2))).unwrap();
    assert_eq!(36, aggregate.average); // NOTE: round((55 + 12 + 42) / 3) = 36.

    let aggregates = sql!(TableAggregateExpr.values(field1).aggregate(average = avg(field2)).filter(average < 20));
    assert_eq!(1, aggregates.len());
    assert_eq!(12, aggregates[0].average); // NOTE: round(12 / 1) = 12.

    let aggregates = sql!(TableAggregateExpr.values(field1).aggregate(avg(field2)).filter(field2_avg < 20));
    assert_eq!(1, aggregates.len());
    assert_eq!(12, aggregates[0].field2_avg); // NOTE: round(12 / 1) = 12.

    let aggregates = sql!(TableAggregateExpr.filter(field2 > 10).values(field1).aggregate(avg(field2)).filter(field2_avg < 20));
    assert_eq!(1, aggregates.len());
    assert_eq!(12, aggregates[0].field2_avg); // NOTE: round(12 / 1) = 12.

    let aggregates = sql!(TableAggregateExpr.filter(field2 > 10).values(field1).aggregate(average = avg(field2)).filter(average < 20));
    assert_eq!(1, aggregates.len());
    assert_eq!(12, aggregates[0].average); // NOTE: round(12 / 1) = 12.

    let value1 = 10;
    let aggregates = sql!(TableAggregateExpr.filter(field2 > value1).values(field1).aggregate(average = avg(field2)).filter(average < 20));
    assert_eq!(1, aggregates.len());
    assert_eq!(12, aggregates[0].average); // NOTE: round(12 / 1) = 12.

    let value2 = 20;
    let aggregates = sql!(TableAggregateExpr.filter(field2 > value1).values(field1).aggregate(average = avg(field2)).filter(average < value2));
    assert_eq!(1, aggregates.len());
    assert_eq!(12, aggregates[0].average); // NOTE: round(12 / 1) = 12.
}
