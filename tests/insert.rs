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

use tql::{ForeignKey, PrimaryKey};

#[SqlTable]
#[allow(dead_code)]
struct Table {
    id: PrimaryKey,
    field1: String,
    field2: i32,
    related_field: ForeignKey<RelatedTable>,
    optional_field: Option<i32>,
}

#[SqlTable]
#[allow(dead_code)]
struct RelatedTable {
    id: PrimaryKey,
    field1: String,
}

#[test]
fn test_insert() {
    assert_eq!(
        "INSERT INTO RelatedTable(field1) VALUES('test') RETURNING id",
        to_sql!(RelatedTable.insert(field1 = "test"))
    );
    assert_eq!(
        "INSERT INTO Table(field1, field2, related_field) VALUES('value1', 55, $1) RETURNING id",
        to_sql!(Table.insert(field1 = "value1", field2 = 55, related_field = related_object))
    );
    assert_eq!(
        "INSERT INTO Table(field1, field2, related_field) VALUES('value1', $1, $2) RETURNING id",
        to_sql!(Table.insert(field1 = "value1", field2 = new_field2, related_field = related_object))
    );
    assert_eq!(
        "INSERT INTO Table(field1, field2, related_field, optional_field) VALUES('value1', 55, $1, 42) RETURNING id",
        to_sql!(Table.insert(field1 = "value1", field2 = 55, related_field = related_object, optional_field = 42))
    );
}
