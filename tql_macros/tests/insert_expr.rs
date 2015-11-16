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
use tql::PrimaryKey;

mod teardown;

use teardown::TearDown;

#[SqlTable]
#[allow(dead_code)]
struct TableInsertExpr {
    primary_key: PrimaryKey,
    field1: String,
    field2: i32,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", &SslMode::None).unwrap()
}

#[test]
fn test_insert() {
    let connection = get_connection();

    let _teardown = TearDown::new(|| {
        let _ = sql!(TableInsertExpr.drop());
    });

    let _ = sql!(TableInsertExpr.drop());

    let result = sql!(TableInsertExpr.insert(field1 = "value1", field2 = 55));
    match result {
        Err(DbError(box db_error)) => assert_eq!(UndefinedTable, *db_error.code()),
        Ok(_) => assert!(false),
        Err(_) => assert!(false),
    }

    let _ = sql!(TableInsertExpr.create());

    let id = sql!(TableInsertExpr.insert(field1 = "value1", field2 = 55)).unwrap();
    assert_eq!(1, id);

    let table = sql!(TableInsertExpr.get(id)).unwrap();
    assert_eq!("value1", table.field1);
    assert_eq!(55, table.field2);

    let new_field2 = 42;
    let id = sql!(TableInsertExpr.insert(field1 = "value2", field2 = new_field2)).unwrap();
    assert_eq!(2, id);

    let table = sql!(TableInsertExpr.get(id)).unwrap();
    assert_eq!("value2", table.field1);
    assert_eq!(42, table.field2);
}
