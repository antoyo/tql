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
use postgres::error::Error::DbError;
use postgres::error::SqlState::UndefinedTable;
use tql::{ForeignKey, PrimaryKey};

mod teardown;

use teardown::TearDown;

#[SqlTable]
struct TableInsertExpr {
    primary_key: PrimaryKey,
    field1: String,
    field2: i32,
    related_field: ForeignKey<RelatedTableInsertExpr>,
    optional_field: Option<i32>,
    boolean: Option<bool>,
    //character: Option<char>, // TODO: does not work.
    float32: Option<f32>,
    float64: Option<f64>,
    //int8: Option<i8>, // TODO: does not work.
    int16: Option<i16>,
    int64: Option<i64>,
}

#[SqlTable]
struct RelatedTableInsertExpr {
    id: PrimaryKey,
    field1: i32,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", &SslMode::None).unwrap()
}

#[test]
fn test_insert() {
    let connection = get_connection();

    let _teardown = TearDown::new(|| {
        let _ = sql!(TableInsertExpr.drop());
        let _ = sql!(RelatedTableInsertExpr.drop());
    });

    let _ = sql!(RelatedTableInsertExpr.create());
    let _ = sql!(TableInsertExpr.drop());

    let related_id = sql!(RelatedTableInsertExpr.insert(field1 = 42)).unwrap();
    let related_field = sql!(RelatedTableInsertExpr.get(related_id)).unwrap();

    let result = sql!(TableInsertExpr.insert(field1 = "value1", field2 = 55, related_field = related_field));
    match result {
        Err(DbError(box db_error)) => assert_eq!(UndefinedTable, *db_error.code()),
        Ok(_) => assert!(false),
        Err(_) => assert!(false),
    }

    let _ = sql!(TableInsertExpr.create());

    let id = sql!(TableInsertExpr.insert(field1 = "value1", field2 = 55, related_field = related_field)).unwrap();
    assert_eq!(1, id);

    let table = sql!(TableInsertExpr.get(id)).unwrap();
    assert_eq!("value1", table.field1);
    assert_eq!(55, table.field2);
    assert!(table.related_field.is_none());
    assert!(table.optional_field.is_none());

    let table = sql!(TableInsertExpr.get(id).join(related_field)).unwrap();
    assert_eq!("value1", table.field1);
    assert_eq!(55, table.field2);
    let related_table = table.related_field.unwrap();
    assert_eq!(related_id, related_table.id);
    assert_eq!(42, related_table.field1);
    assert!(table.optional_field.is_none());

    let new_field2 = 42;
    let id = sql!(TableInsertExpr.insert(field1 = "value2", field2 = new_field2, related_field = related_field)).unwrap();
    assert_eq!(2, id);

    let table = sql!(TableInsertExpr.get(id)).unwrap();
    assert_eq!("value2", table.field1);
    assert_eq!(42, table.field2);
    assert!(table.related_field.is_none());
    assert!(table.optional_field.is_none());

    let new_field1 = "value3".to_owned();
    let new_field2 = 24;
    let id = sql!(TableInsertExpr.insert(field1 = new_field1, field2 = new_field2, related_field = related_field, optional_field = 12)).unwrap();
    assert_eq!(3, id);

    let table = sql!(TableInsertExpr.get(id)).unwrap();
    assert_eq!("value3", table.field1);
    assert_eq!(24, table.field2);
    assert!(table.related_field.is_none());
    assert_eq!(Some(12), table.optional_field);

    let boolean_value = true;
    //let character = 'a';
    let float32 = 3.14f32;
    let float64 = 3.14f64;
    //let int8 = 42i8;
    let int16 = 42i16;
    let int64 = 42i64;
    let id = sql!(TableInsertExpr.insert(field1 = new_field1, field2 = new_field2, related_field = related_field, optional_field = 12, boolean = boolean_value, /*character = character,*/ float32 = float32, float64 = float64, /*int8 = int8,*/ int16 = int16, int64 = int64)).unwrap();
    assert_eq!(4, id);
}
