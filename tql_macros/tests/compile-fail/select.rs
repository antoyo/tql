// error-pattern: use of unsupported type name `&'a str` [E0412]
// error-pattern: run `rustc --explain E0412` to see a detailed explanation
// error-pattern: attempted access of field `field2` on type `Table`, but no field with that name was found
// error-pattern: did you mean field1?
// error-pattern: attempted access of field `field2` on type `Table`, but no field with that name was found
// error-pattern: did you mean field1?

#![feature(plugin)]
#![plugin(tql_macros)]

#[SqlTable]
struct Table<'a> {
    field1: String,
    string: &'a str,
}

fn main() {
    sql!(Table.filter(field1 == "value1" && field2 < 100).sort(-field2));
}
