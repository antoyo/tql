#![feature(plugin)]
#![plugin(tql)]

#[sql_table]
struct Table<'a> {
    field1: &'a str,
}

#[test]
fn test_select() {
    assert_eq!("SELECT * FROM Table", to_sql!(Table.collect()));
    assert_eq!("SELECT * FROM Table WHERE field1 = ? AND field2 < 100 ORDER BY field2 DESC",
               to_sql!(Table.filter(field1 == "value1" && field2 < 100).sort(-field2)));
}
