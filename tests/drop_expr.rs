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
use postgres::error::UNDEFINED_TABLE;
use tql::PrimaryKey;
use tql_macros::sql;

mod teardown;

use teardown::TearDown;

#[derive(SqlTable)]
#[allow(dead_code)]
struct TableDropExpr {
    primary_key: PrimaryKey,
    field1: String,
    field2: i32,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", TlsMode::None).unwrap()
}

#[test]
fn test_drop() {
    let connection = get_connection();

    let _teardown = TearDown::new(|| {
        let _ = TableDropExpr.drop();
    });

    let _ = TableDropExpr::create();

    let result = sql!(TableDropExpr.insert(field1 = "value1", field2 = 55));
    assert!(result.is_ok());

    let _ = TableDropExpr.drop();

    let result = sql!(TableDropExpr.insert(field1 = "value1", field2 = 55));
    match result {
        Err(db_error) => assert_eq!(Some(&UNDEFINED_TABLE), db_error.code()),
        Ok(_) => assert!(false),
    }
}
