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

#[macro_use]
extern crate lazy_static;
extern crate postgres;
extern crate tql;
#[macro_use]
extern crate tql_macros;

mod teardown;

use std::sync::Mutex;

use postgres::{Connection, TlsMode};
use tql::{ForeignKey, PrimaryKey};
use tql_macros::sql;

use teardown::TearDown;

#[derive(SqlTable)]
#[allow(dead_code)]
struct TableUpdateExpr {
    id: PrimaryKey,
    field1: String,
    field2: i32,
    field3: i32,
    related_field: ForeignKey<RelatedTable>,
}

#[derive(SqlTable)]
#[allow(dead_code)]
struct RelatedTable {
    id: PrimaryKey,
    field1: String,
}

lazy_static! {
    static ref LOCK: Mutex<Connection> = Mutex::new(get_connection());
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", TlsMode::None).unwrap()
}

#[test]
fn test_update() {
    let connection = LOCK.lock().unwrap();

    let _teardown = TearDown::new(|| {
        let _ = TableUpdateExpr::drop(&connection);
        let _ = RelatedTable::drop(&connection);
    });

    let _ = RelatedTable::create(&connection);
    let _ = TableUpdateExpr::create(&connection);

    let id = sql!(RelatedTable.insert(field1 = "")).unwrap();
    let related_field = sql!(RelatedTable.get(id)).unwrap();

    let id = sql!(TableUpdateExpr.insert(field1 = "", field2 = 0, field3 = 0, related_field = related_field)).unwrap();

    let num_updated = sql!(TableUpdateExpr.get(id).update(field1 = "value1", field2 = 55)).unwrap();
    assert_eq!(1, num_updated);

    let table = sql!(TableUpdateExpr.get(id)).unwrap();
    assert_eq!(id, table.id);
    assert_eq!("value1", table.field1);
    assert_eq!(55, table.field2);

    let new_field2 = 42;
    let num_updated = sql!(TableUpdateExpr.filter(id == id).update(field1 = "test", field2 = new_field2)).unwrap();
    assert_eq!(1, num_updated);

    let table = sql!(TableUpdateExpr.get(id)).unwrap();
    assert_eq!(id, table.id);
    assert_eq!("test", table.field1);
    assert_eq!(42, table.field2);

    let new_id = sql!(TableUpdateExpr
        .insert(field1 = "", field2 = 0, field3 = 0, related_field = related_field)).unwrap();

    let num_updated = sql!(TableUpdateExpr
        .filter(id > new_id)
        .update(field1 = "test", field2 = new_field2)).unwrap();
    assert_eq!(0, num_updated);

    let my_string = "my string";
    let new_field2 = 24;
    let num_updated = sql!(TableUpdateExpr
           .filter(id >= id && id <= new_id)
           .update(field1 = my_string, field2 = new_field2)).unwrap();
    assert_eq!(2, num_updated);

    let table = sql!(TableUpdateExpr.get(id)).unwrap();
    assert_eq!(id, table.id);
    assert_eq!("my string", table.field1);
    assert_eq!(24, table.field2);

    let table = sql!(TableUpdateExpr.get(new_id)).unwrap();
    assert_eq!(new_id, table.id);
    assert_eq!("my string", table.field1);
    assert_eq!(24, table.field2);
}

#[test]
fn test_update_operation() {
    let connection = LOCK.lock().unwrap();

    let _teardown = TearDown::new(|| {
        let _ = TableUpdateExpr::drop(&connection);
        let _ = RelatedTable::drop(&connection);
    });

    let _ = RelatedTable::create(&connection);
    let _ = TableUpdateExpr::create(&connection);

    let id = sql!(RelatedTable.insert(field1 = "")).unwrap();
    let related_field = sql!(RelatedTable.get(id)).unwrap();

    let id = sql!(TableUpdateExpr.insert(field1 = "", field2 = 0, field3 = 1, related_field = related_field)).unwrap();

    let num_updated = sql!(TableUpdateExpr.get(id).update(field2 += 10)).unwrap();
    assert_eq!(1, num_updated);

    let table = sql!(TableUpdateExpr.get(id)).unwrap();
    assert_eq!(id, table.id);
    assert_eq!("", table.field1);
    assert_eq!(10, table.field2);
    assert_eq!(1, table.field3);

    let num_updated = sql!(TableUpdateExpr.get(id).update(field2 -= 3)).unwrap();
    assert_eq!(1, num_updated);

    let table = sql!(TableUpdateExpr.get(id)).unwrap();
    assert_eq!(id, table.id);
    assert_eq!("", table.field1);
    assert_eq!(7, table.field2);
    assert_eq!(1, table.field3);

    let num_updated = sql!(TableUpdateExpr.get(id).update(field2 *= 2)).unwrap();
    assert_eq!(1, num_updated);

    let table = sql!(TableUpdateExpr.get(id)).unwrap();
    assert_eq!(id, table.id);
    assert_eq!("", table.field1);
    assert_eq!(14, table.field2);
    assert_eq!(1, table.field3);

    let num_updated = sql!(TableUpdateExpr.get(id).update(field2 /= 3)).unwrap();
    assert_eq!(1, num_updated);

    let table = sql!(TableUpdateExpr.get(id)).unwrap();
    assert_eq!(id, table.id);
    assert_eq!("", table.field1);
    assert_eq!(4, table.field2);
    assert_eq!(1, table.field3);

    let num_updated = sql!(TableUpdateExpr.get(id).update(field2 += 10, field3 *= 3)).unwrap();
    assert_eq!(1, num_updated);

    let table = sql!(TableUpdateExpr.get(id)).unwrap();
    assert_eq!(id, table.id);
    assert_eq!("", table.field1);
    assert_eq!(14, table.field2);
    assert_eq!(3, table.field3);

    let num_updated = sql!(TableUpdateExpr.get(id).update(field2 %= 7)).unwrap();
    assert_eq!(1, num_updated);

    let table = sql!(TableUpdateExpr.get(id)).unwrap();
    assert_eq!(id, table.id);
    assert_eq!("", table.field1);
    assert_eq!(0, table.field2);
    assert_eq!(3, table.field3);
}
