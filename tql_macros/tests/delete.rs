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

use tql::PrimaryKey;

#[SqlTable]
#[allow(dead_code)]
struct Table {
    id: PrimaryKey,
    field1: String,
    field2: i32,
}

#[test]
fn test_delete() {
    //assert_eq!(
        //"DELETE FROM Table",
        //to_sql!(Table.delete()) // TODO: this does not work because the errors (including
        //warnings) return a dummy result.
    //);
    assert_eq!(
        "DELETE FROM Table WHERE field1 = 'test'",
        to_sql!(Table.filter(field1 == "test").delete())
    );
}
