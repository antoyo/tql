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
struct TableDropExpr {
    primary_key: PrimaryKey,
    field1: String,
    field2: i32,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", &SslMode::None).unwrap()
}

#[test]
fn test_drop() {
    let connection = get_connection();

    let _teardown = TearDown::new(|| {
        let _ = sql!(TableDropExpr.drop());
    });

    let _ = sql!(TableDropExpr.create());

    let result = sql!(TableDropExpr.insert(field1 = "value1", field2 = 55));
    assert!(result.is_ok());

    let _ = sql!(TableDropExpr.drop());

    let result = sql!(TableDropExpr.insert(field1 = "value1", field2 = 55));
    match result {
        Err(DbError(box db_error)) => assert_eq!(UndefinedTable, *db_error.code()),
        Ok(_) => assert!(false),
        Err(_) => assert!(false),
    }
}
