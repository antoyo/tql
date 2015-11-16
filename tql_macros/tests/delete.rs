#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use tql::PrimaryKey;

#[SqlTable]
#[allow(dead_code)]
struct Table {
    id: PrimaryKey,
    field1: String,
    field2: i32,
}

#[test]
fn test_delete() {
    //assert_eq!(
        //"DELETE FROM Table",
        //to_sql!(Table.delete()) // TODO: this does not work because the errors (including
        //warnings) return a dummy result.
    //);
    assert_eq!(
        "DELETE FROM Table WHERE field1 = 'test'",
        to_sql!(Table.filter(field1 == "test").delete())
    );
}
