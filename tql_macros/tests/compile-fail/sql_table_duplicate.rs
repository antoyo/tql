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

//! Tests of the syntax extension errors.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate tql;

#[SqlTable]
struct Table {
    field1: String,
}

#[SqlTable]
struct Table {
    //~^ ERROR duplicate definition of table `Table` [E0428]
    //~| HELP run `rustc --explain E0428` to see a detailed explanation
}
