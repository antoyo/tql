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

//! Tests of the type analyzer lint for the `#[SqlTable]` attribute.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use tql::{ForeignKey, PrimaryKey};

struct Connection {
    value: String,
}

#[SqlTable]
struct Table {
    //~^ WARNING No primary key found
    field1: String,
    related_field1: ForeignKey<Connection>,
    //~^ ERROR `Connection` does not name an SQL table [E0422]
    //~| HELP run `rustc --explain E0422` to see a detailed explanation
    //~| HELP did you forget to add the #[SqlTable] attribute on the Connection struct?
    related_field2: ForeignKey<RelatedTable>,
}

#[SqlTable]
struct RelatedTable {
    id: PrimaryKey,
}
