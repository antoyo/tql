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

//! Tests of the macro.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate tql;

use tql::{ForeignKey, PrimaryKey};

struct Connection {
    value: String,
}

#[SqlTable]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
    field2: ForeignKey<Table>,
}

fn main() {
    to_sql!();
    //~^ ERROR this macro takes 1 parameter but 0 parameters were supplied [E0061]

    sql!();
    //~^ ERROR this macro takes 1 parameter but 0 parameters were supplied [E0061]

    sql!(Table);
    //~^ ERROR `Table` is the name of a struct, but this expression uses it like a method name [E0423]
    //~| HELP run `rustc --explain E0423` to see a detailed explanation
    //~| HELP did you mean to write `Table.method()`?

    sql!(Table());
    //~^ ERROR Expected method call

    sql!(Table.insert().filter(i32_field == 10).delete());
    //~^ ERROR cannot call the filter() method with the insert() method
    //~| ERROR cannot call the delete() method with the insert() method

    sql!(Table.update(i32_field = 10).filter(i32_field == 10).delete());
    //~^ ERROR cannot call the delete() method with the update() method

    sql!(Table.join(field2).filter(i32_field == 10).delete());
    //~^ ERROR cannot call the join() method with the delete() method

    sql!(Table.create().insert().filter(i32_field == 10).delete());
    //~^ ERROR cannot call the insert() method with the create() method
    //~| ERROR cannot call the filter() method with the create() method
    //~| ERROR cannot call the delete() method with the create() method

    sql!(Table.drop().insert().filter(i32_field == 10).delete());
    //~^ ERROR cannot call the insert() method with the drop() method
    //~| ERROR cannot call the filter() method with the drop() method
    //~| ERROR cannot call the delete() method with the drop() method

    sql!(Table.filter(i32_field == 10).aggregate(avg(i32_field)).drop().insert().filter(i32_field_avg == 10).delete());
    //~^ ERROR cannot call the drop() method with the aggregate() method
    //~| ERROR cannot call the insert() method with the aggregate() method
    //~| ERROR cannot call the delete() method with the aggregate() method
}
