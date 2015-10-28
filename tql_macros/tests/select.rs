#![feature(plugin)]
#![plugin(tql_macros)]

#[SqlTable]
struct Table {
    field1: String,
}

#[test]
fn test_select() {
    assert_eq!("SELECT Table.field1 FROM Table", to_sql!(Table.all()));
}
