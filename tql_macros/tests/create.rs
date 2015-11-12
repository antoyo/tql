#![feature(plugin)]
#![plugin(tql_macros)]

extern crate chrono;
extern crate postgres;
extern crate tql;

use chrono::datetime::DateTime;
use chrono::naive::date::NaiveDate;
use chrono::naive::datetime::NaiveDateTime;
use chrono::naive::time::NaiveTime;
use chrono::offset::local::Local;
use chrono::offset::utc::UTC;
use tql::{ForeignKey, PrimaryKey};

#[SqlTable]
struct Table {
    id: PrimaryKey,
    field1: String,
    field2: i32,
    field3: Option<i32>,
    related_field: ForeignKey<RelatedTable>,
}

#[SqlTable]
struct RelatedTable {
    id: PrimaryKey,
    field1: String,
}

#[SqlTable]
struct Dates {
    pk: PrimaryKey,
    date1: NaiveDateTime,
    date2: DateTime<UTC>,
    date3: DateTime<Local>,
    date4: NaiveDate,
    date5: NaiveTime,
}

#[SqlTable]
struct OtherTypes {
    pk: PrimaryKey,
    boolean: bool,
    bytestring: Vec<u8>,
    character: char,
    float32: f32,
    float64: f64,
    int8: i8,
    int16: i16,
    int32: i32,
    int64: i64,
}

#[test]
fn test_create() {
    assert_eq!(
        "CREATE TABLE Table (field1 CHARACTER VARYING NOT NULL, field2 INTEGER NOT NULL, field3 INTEGER, id SERIAL PRIMARY KEY NOT NULL, related_field INTEGER REFERENCES RelatedTable(id) NOT NULL)",
        to_sql!(Table.create())
    );
    assert_eq!(
        "CREATE TABLE RelatedTable (field1 CHARACTER VARYING NOT NULL, id SERIAL PRIMARY KEY NOT NULL)",
        to_sql!(RelatedTable.create())
    );
    assert_eq!(
        "CREATE TABLE Dates (date1 TIMESTAMP NOT NULL, date2 TIMESTAMP WITH TIME ZONE NOT NULL, date3 TIMESTAMP WITH TIME ZONE NOT NULL, date4 DATE NOT NULL, date5 TIME NOT NULL, pk SERIAL PRIMARY KEY NOT NULL)",
        to_sql!(Dates.create())
    );
    assert_eq!(
        "CREATE TABLE OtherTypes (boolean BOOLEAN NOT NULL, bytestring BYTEA NOT NULL, character CHARACTER(1) NOT NULL, float32 REAL NOT NULL, float64 DOUBLE PRECISION NOT NULL, int16 SMALLINT NOT NULL, int32 INTEGER NOT NULL, int64 BIGINT NOT NULL, int8 CHARACTER(1) NOT NULL, pk SERIAL PRIMARY KEY NOT NULL)",
        to_sql!(OtherTypes.create())
    );
}
