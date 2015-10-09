#![feature(plugin)]
#![plugin(tql)]

#[sql_table]
struct Table<'a> {
    field1: &'a str,
}

#[test]
fn test_select() {
    assert_eq!("SELECT field1 FROM Table", to_sql!(Table.collect()));
    // TODO: cela devrait échouer avoir une erreur.
    assert_eq!("SELECT field1 FROM Table WHERE field1 = 'value1' AND field2 < 100 ORDER BY field2 DESC",
               to_sql!(Table.filter(field1 == "value1" && field2 < 100).sort(-field2)));
}
