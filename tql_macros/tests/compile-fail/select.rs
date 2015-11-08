//! Tests of the methods related to `Query::Select`.

#![feature(plugin)]
#![plugin(tql_macros)]

extern crate tql;

use tql::PrimaryKey;

struct Connection {
    value: String,
}

#[SqlTable]
struct Table {
    id: PrimaryKey,
    field1: String,
    i32_field: i32,
}

fn main() {
    sql!(Table.filter(field1 == "value1" && field2 < 100).sort(-field2));
    //~^ ERROR attempted access of field `field2` on type `Table`, but no field with that name was found
    //~| HELP did you mean field1?
    //~| ERROR attempted access of field `field2` on type `Table`, but no field with that name was found
    //~| HELP did you mean field1?

    sql!(Table.filter(field1 == "value1" && i32_field < 100u32));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `u32` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(field1 == "value1" && i32_field < 100u32).sort(-i32_field));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `u32` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(field1 == "value1" && i32_field < 100u64));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `u64` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(field1 == "value1" && i32_field < 100i8));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `i8` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(i32_field >= b'f' || field1 == 't'));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `u8` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)
    //~| ERROR mismatched types:
    //~| expected `String`,
    //~| found `char` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(i32_field >= b"test"));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `Vec<u8>` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(i32_field >= r#""test""#));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `String` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(i32_field >= 3.141592f32));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `f32` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(i32_field >= 3.141592f64));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `f64` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(i32_field >= 3.141592));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `floating-point variable` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(i32_field >= 42).sort(fild1));
    //~^ ERROR attempted access of field `fild1` on type `Table`, but no field with that name was found
    //~| HELP did you mean field1?

    sql!(Table.filter(i32_field >= 42).sort(-fild1));
    //~^ ERROR attempted access of field `fild1` on type `Table`, but no field with that name was found
    //~| HELP did you mean field1?

    sql!(Table.filter(i32_fiel >= 42));
    //~^ ERROR attempted access of field `i32_fiel` on type `Table`, but no field with that name was found
    //~| HELP did you mean i32_field?

    sql!(Table.filter(i32_field == true));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `bool` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(i32_field == false));
    //~^ ERROR mismatched types:
    //~| expected `i32`,
    //~| found `bool` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.all()[.."auinesta"]);
    //~^ ERROR mismatched types:
    //~| expected `i64`,
    //~| found `String` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.all()[true..false]);
    //~^ ERROR mismatched types:
    //~| expected `i64`,
    //~| found `bool` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)
    //~| ERROR mismatched types:
    //~| expected `i64`,
    //~| found `bool` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    sql!(Table.filter(i32_field < 100 && field1 == "value1").sort(*i32_field, *field1));
    //~^ ERROR Expected - or identifier
    //~| ERROR Expected - or identifier

    sql!(Tble.filter(field1 == "value"));
    //~^ ERROR `Tble` does not name an SQL table [E0422]
    //~| HELP run `rustc --explain E0422` to see a detailed explanation
    //~| HELP did you mean Table?

    sql!(TestTable.flter(field1 == "value"));
    //~^ ERROR `TestTable` does not name an SQL table [E0422]
    //~| HELP run `rustc --explain E0422` to see a detailed explanation
    //~| HELP did you forget to add the #[sql_table] attribute on the TestTable struct?
    //~| ERROR no method named `flter` found in tql
    //~| HELP did you mean filter?

    sql!(Table.all(id == 1));
    //~^ ERROR this method takes 0 parameters but 1 parameter was supplied [E0061]
    //~| HELP run `rustc --explain E0061` to see a detailed explanation

    sql!(Table.all().join(test));
    //~^ ERROR attempted access of field `test` on type `Table`, but no field with that name was found

    sql!(Table.all().join(field));
    //~^ ERROR attempted access of field `field` on type `Table`, but no field with that name was found
    //~| HELP did you mean field1?

    sql!(Table.all().join(field1, i32_field));
    //~^ ERROR mismatched types:
    //~| expected `ForeignKey<_>`,
    //~| found `String` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)
    //~| ERROR mismatched types:
    //~| expected `ForeignKey<_>`,
    //~| found `i32` [E0308]
    //~| HELP run `rustc --explain E0308` to see a detailed explanation
    //~| NOTE in this expansion of sql! (defined in tql)

    //to_sql!(Table.all().join(address, address)); // TODO: devrait causer une erreur.
}
