#![feature(plugin)]
#![plugin(tql)]

#[sql_table]
struct Table<'a> {
    field1: &'a str,
}

#[test]
fn test_select() {
    assert_eq!("SELECT * FROM Table", sql!(Table.collect()));
}
