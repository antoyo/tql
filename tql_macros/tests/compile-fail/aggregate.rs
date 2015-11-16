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

//! Tests of the aggregate() method.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate tql;

use tql::PrimaryKey;

#[SqlTable]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
}

fn main() {
    sql!(Table.aggregate(avh(i32_field)));
    //~^ ERROR unresolved name `avh` [E0425]
    //~| HELP run `rustc --explain E0425` to see a detailed explanation
    //~| HELP did you mean avg?

    sql!(Table.values(test).aggregate(avg(i32_field)));
    //~^ ERROR attempted access of field `test` on type `Table`, but no field with that name was found

    sql!(Table.values("test").aggregate(avg(i32_field)));
    //~^ ERROR Expected identifier

    sql!(Table.aggregate(avg(i32_field, field1)));
    //~^ ERROR this function takes 1 parameter but 2 parameters were supplied [E0061]

    sql!(Table.values(i32_field).aggregate(average = avg(i32_field)).filter(avg < 20));
    //~^ ERROR no aggregate field named `avg` found

    //sql!(Table.values(i32_field).aggregate(average = avg(i32_field)).filter(avrage < 20));
    // TODO: propose similar names.

    if let Some(aggregate) = sql!(Table.aggregate(average = avg(field2))) {
        println!("{}", aggregate.averag);
    }
}
