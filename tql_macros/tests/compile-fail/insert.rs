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

//! Tests of the insert() method.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate tql;

use tql::{ForeignKey, PrimaryKey};

#[SqlTable]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
    field2: String,
    related_field: ForeignKey<RelatedTable>,
}

#[SqlTable]
struct RelatedTable {
    id: PrimaryKey,
}

fn main() {
    sql!(Table.insert(field1 = 42, i32_field = 91));
    //~^ ERROR missing fields: `field2`, `related_field` [E0063]
    //~| HELP run `rustc --explain E0063` to see a detailed explanation

    sql!(Table.insert(field1 = 42, i32_fild = 91));
    //~^ ERROR attempted access of field `i32_fild` on type `Table`, but no field with that name was found
    //~| HELP did you mean i32_field?

    sql!(Table.insert(i32_field += 42, field1 = "Test"));
    //~^ ERROR expected = but got +=
    //~| ERROR missing fields: `field2`, `related_field` [E0063]

    sql!(Table.insert(i32_field = 42, field1 -= "Test"));
    //~^ ERROR expected = but got -=
    //~| ERROR missing fields: `field2`, `related_field` [E0063]

    let related_field = RelatedTable {
        id: 1,
    };
    sql!(Table.insert(field1 = 42, i32_field = 91, field2 = "test", related_field = related_field));
    //~^ ERROR mismatched types:
    //~| expected `String`,
    //~| found `integral variable` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.insert(field1 = "test", i32_field = 91, field2 = "test", related_field = 1));
    //~^ ERROR mismatched types:
    //~| expected `RelatedTable`,
    //~| found `integral variable` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)
}
