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

extern crate chrono;
extern crate rusqlite;
extern crate tql;
#[macro_use]
extern crate tql_macros;

use chrono::DateTime;
use chrono::naive::{NaiveDate, NaiveDateTime, NaiveTime};
use chrono::offset::{Local, Utc};
use tql::{ForeignKey, PrimaryKey};
use tql_macros::to_sql;

#[derive(SqlTable)]
#[allow(dead_code)]
struct Table {
    id: PrimaryKey,
    field1: String,
    field2: i32,
    field3: Option<i32>,
    related_field: ForeignKey<RelatedTable>,
}

#[derive(SqlTable)]
#[allow(dead_code)]
struct RelatedTable {
    id: PrimaryKey,
    field1: String,
}

#[derive(SqlTable)]
#[allow(dead_code)]
struct Dates {
    pk: PrimaryKey,
    date1: NaiveDateTime,
    date2: DateTime<Utc>,
    date3: DateTime<Local>,
    date4: NaiveDate,
    date5: NaiveTime,
}

#[derive(SqlTable)]
#[allow(dead_code)]
struct OtherTypes {
    pk: PrimaryKey,
    boolean: bool,
    bytestring: Vec<u8>,
    //character: char, // FIXME: not working.
    float64: f64,
    int8: i8,
    int16: i16,
    int32: i32,
    int64: i64,
}

#[test]
fn test_create() {
    assert_eq!(
        "CREATE TABLE Table (id INTEGER PRIMARY KEY NOT NULL, field1 CHARACTER VARYING NOT NULL, field2 INTEGER NOT NULL, field3 INTEGER, related_field INTEGER REFERENCES RelatedTable(id) NOT NULL)",
        to_sql!(Table.create())
    );
    assert_eq!(
        "CREATE TABLE RelatedTable (id INTEGER PRIMARY KEY NOT NULL, field1 CHARACTER VARYING NOT NULL)",
        to_sql!(RelatedTable.create())
    );
    assert_eq!(
        "CREATE TABLE Dates (pk INTEGER PRIMARY KEY NOT NULL, date1 TIMESTAMP NOT NULL, date2 TIMESTAMP WITH TIME ZONE NOT NULL, date3 TIMESTAMP WITH TIME ZONE NOT NULL, date4 DATE NOT NULL, date5 TIME NOT NULL)",
        to_sql!(Dates.create())
    );
    assert_eq!(
        "CREATE TABLE OtherTypes (pk INTEGER PRIMARY KEY NOT NULL, boolean BOOLEAN NOT NULL, bytestring BYTEA NOT NULL, float64 DOUBLE PRECISION NOT NULL, int8 CHARACTER(1) NOT NULL, int16 SMALLINT NOT NULL, int32 INTEGER NOT NULL, int64 BIGINT NOT NULL)",
        to_sql!(OtherTypes.create())
    );
}
