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

//! Tests of the `#[SqlTable]` attribute.

#![feature(plugin)]
#![plugin(tql_macros)]

struct Connection {
    value: String,
}

#[SqlTable]
struct Table<'a> {
    string: &'a str,
    //~^ ERROR use of unsupported type name `&'a str` [E0412]
    //~| run `rustc --explain E0412` to see a detailed explanation
    connection: Connection,
    //~^ ERROR use of unsupported type name `Connection` [E0412]
    //~| run `rustc --explain E0412` to see a detailed explanation
    connection2: Option<Connection>,
    //~^ ERROR use of unsupported type name `Connection` [E0412]
    //~| run `rustc --explain E0412` to see a detailed explanation
    nested_options: Option<Option<String>>,
    //~^ ERROR use of unsupported type name `Option<String>` [E0412]
    //~| run `rustc --explain E0412` to see a detailed explanation
    datetime: DateTime,
    //~^ ERROR use of unsupported type name `DateTime` [E0412]
    //~| run `rustc --explain E0412` to see a detailed explanation
    datetime_i32: DateTime<i32>,
    //~^ ERROR use of unsupported type name `DateTime<i32>` [E0412]
    //~| run `rustc --explain E0412` to see a detailed explanation
    foreign_value: ForeignKey,
    //~^ ERROR use of unsupported type name `ForeignKey` [E0412]
    //~| run `rustc --explain E0412` to see a detailed explanation
    optional_value: Option,
    //~^ ERROR use of unsupported type name `Option` [E0412]
    //~| run `rustc --explain E0412` to see a detailed explanation
    vector: Vec,
    //~^ ERROR use of unsupported type name `Vec` [E0412]
    //~| run `rustc --explain E0412` to see a detailed explanation
    vector_i32: Vec<i32>,
    //~^ ERROR use of unsupported type name `Vec<i32>` [E0412]
    //~| run `rustc --explain E0412` to see a detailed explanation
}
