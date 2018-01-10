/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Tests of the macro.

#![feature(proc_macro)]

extern crate tql;
#[macro_use]
extern crate tql_macros;

use tql::{ForeignKey, PrimaryKey};
use tql_macros::sql;

struct Connection {
    value: String,
}

#[derive(SqlTable)]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
    field2: ForeignKey<Table>,
}

fn main() {
    to_sql!();
    //~^ ERROR failed to parse expression: failed to parse
    // FIXME: when it becomes stable.
    // ~^ ERROR this macro takes 1 parameter but 0 parameters were supplied [E0061]

    sql!();
    //~^ ERROR failed to parse expression: failed to parse
    // FIXME: when it becomes stable.
    // ~^ ERROR this macro takes 1 parameter but 0 parameters were supplied [E0061]

    sql!(Table);
    //~^ ERROR `Table` is the name of a struct, but this expression uses it like a method name
    //~| HELP did you mean to write `Table.method()`?

    sql!(Table());
    //~^ ERROR Expected method call

    sql!(Table.insert().filter(i32_field == 10).delete());
    //~^ ERROR cannot call the filter() method with the insert() method
    //~| ERROR cannot call the delete() method with the insert() method

    sql!(Table.update(i32_field = 10).filter(i32_field == 10).delete());
    //~^ ERROR cannot call the delete() method with the update() method

    sql!(Table.join(field2 = Table { id, field1, i32_field, field2 }).filter(i32_field == 10).delete());
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
