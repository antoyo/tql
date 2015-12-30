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

//! Tests of the methods available in the filter() method.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate tql;

use chrono::datetime::DateTime;
use chrono::offset::utc::UTC;
use tql::PrimaryKey;

#[SqlTable]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
    date: DateTime<UTC>,
    option_field: Option<i32>,
}

fn main() {
    sql!(Table.filter(i32_field.year() == 2015));
    //~^ ERROR no method named `year` found for type `i32`

    sql!(Table.filter(date.test() == 2015));
    //~^ ERROR no method named `test` found for type `chrono::datetime::DateTime<chrono::offset::utc::UTC>`

    sql!(Table.filter(date.yar() == 2015));
    //~^ ERROR no method named `yar` found for type `chrono::datetime::DateTime<chrono::offset::utc::UTC>`
    //~| HELP did you mean year?

    sql!(Table.filter(dte.year() == 2015));
    //~^ ERROR attempted access of field `dte` on type `Table`, but no field with that name was found
    //~| HELP did you mean date?

    sql!(Table.filter(date.year()));
    //~^ ERROR mismatched types:
    //~| expected `bool`,
    //~| found `i32` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(field1.ends_with(1) == true));
    //~^ ERROR mismatched types:
    //~| expected `String`,
    //~| found `integral variable` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(field1.len() == "test"));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `String` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(field1.len()));
    //~^ ERROR mismatched types:
    //~| expected `bool`,
    //~| found `i32` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(field1.len() && option_field.is_some()));
    //~^ ERROR mismatched types:
    //~| expected `bool`,
    //~| found `i32` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)
}
