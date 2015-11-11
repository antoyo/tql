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
    // TODO: proposer des noms similaires.

    if let Some(aggregate) = sql!(Table.aggregate(average = avg(field2))) {
        println!("{}", aggregate.averag);
    }
}
