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

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

mod teardown;

use postgres::{Connection, SslMode};
use tql::PrimaryKey;

use teardown::TearDown;

#[SqlTable]
#[allow(dead_code)]
struct TableDeleteExpr {
    id: PrimaryKey,
    field1: String,
    field2: i32,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", &SslMode::None).unwrap()
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
