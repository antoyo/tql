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
    field3: i32,
    related_field: ForeignKey<RelatedTable>,
}

#[SqlTable]
#[allow(dead_code)]
struct RelatedTable {
    id: PrimaryKey,
    field1: String,
}

#[test]
fn test_update() {
    assert_eq!(
        "UPDATE Table SET field1 = 'value1', field2 = 55 WHERE id = 1",
        to_sql!(Table.get(1).update(field1 = "value1", field2 = 55))
    );
    assert_eq!(
        "UPDATE Table SET field1 = 'value1', field2 = $1 WHERE id = 1",
        to_sql!(Table.filter(id == 1).update(field1 = "value1", field2 = new_field2))
    );
}

#[test]
fn test_update_operation() {
    assert_eq!(
        "UPDATE Table SET field2 = field2 + 1 WHERE id = 1",
        to_sql!(Table.get(1).update(field2 += 1))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 - 3 WHERE id = 1",
        to_sql!(Table.get(1).update(field2 -= 3))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 % 7 WHERE id = 1",
        to_sql!(Table.get(1).update(field2 %= 7))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 * 2 WHERE id = 1",
        to_sql!(Table.get(1).update(field2 *= 2))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 / 3 WHERE id = 1",
        to_sql!(Table.get(1).update(field2 /= 3))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 + 10, field3 = field3 / 3 WHERE id = 1",
        to_sql!(Table.get(1).update(field2 += 10, field3 /= 3))
    );
}
