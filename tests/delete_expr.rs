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

extern crate tql;
#[macro_use]
extern crate tql_macros;

#[macro_use]
mod connection;
mod teardown;

backend_extern_crate!();

use tql::PrimaryKey;
use tql_macros::sql;

use connection::get_connection;
use teardown::TearDown;

#[derive(SqlTable)]
#[allow(dead_code)]
struct TableDeleteExpr {
    id: PrimaryKey,
    field1: String,
    field2: i32,
}

#[test]
fn test_delete() {
    let connection = get_connection();

    let _teardown = TearDown::new(|| {
        let _ = sql!(TableDeleteExpr.drop());
    });

    let _ = sql!(TableDeleteExpr.create());

    let id = sql!(TableDeleteExpr.insert(field1 = "", field2 = 0)).unwrap();

    let table = sql!(TableDeleteExpr.get(id));
    assert!(table.is_some());

    //assert_eq!(
        //"DELETE FROM TableDeleteExpr",
        //to_sql!(Table.delete()) // TODO: this does not work because the errors (including
        //warnings) return a dummy result.
    //);

    let num_deleted = sql!(TableDeleteExpr.filter(field1 == "").delete()).unwrap();
    assert_eq!(1, num_deleted);

    let table = sql!(TableDeleteExpr.get(id));
    assert!(table.is_none());

    let id1 = sql!(TableDeleteExpr.insert(field1 = "", field2 = 1)).unwrap();
    let id2 = sql!(TableDeleteExpr.insert(field1 = "", field2 = 2)).unwrap();
    let id3 = sql!(TableDeleteExpr.insert(field1 = "", field2 = 3)).unwrap();

    let table = sql!(TableDeleteExpr.get(id1));
    assert!(table.is_some());

    let table = sql!(TableDeleteExpr.get(id2));
    assert!(table.is_some());

    let table = sql!(TableDeleteExpr.get(id3));
    assert!(table.is_some());

    let num_deleted = sql!(TableDeleteExpr.filter(field2 < 5).delete()).unwrap();
    assert_eq!(3, num_deleted);

    let table = sql!(TableDeleteExpr.get(id1));
    assert!(table.is_none());

    let table = sql!(TableDeleteExpr.get(id2));
    assert!(table.is_none());

    let table = sql!(TableDeleteExpr.get(id3));
    assert!(table.is_none());

    let id = sql!(TableDeleteExpr.insert(field1 = "", field2 = 1)).unwrap();

    let num_deleted = sql!(TableDeleteExpr.filter(field2 > 5).delete()).unwrap();
    assert_eq!(0, num_deleted);

    let table = sql!(TableDeleteExpr.get(id));
    assert!(table.is_some());

    let num_deleted = sql!(TableDeleteExpr.get(id).delete()).unwrap();
    assert_eq!(1, num_deleted);

    let table = sql!(TableDeleteExpr.get(id));
    assert!(table.is_none());
}
