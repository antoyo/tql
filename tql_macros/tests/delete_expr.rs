#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use postgres::{Connection, SslMode};
use tql::PrimaryKey;

#[SqlTable]
#[allow(dead_code)]
struct TableDeleteExpr {
    id: PrimaryKey,
    field1: String,
    field2: i32,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", &SslMode::None).unwrap()
}

#[test]
fn test_delete() {
    let connection = get_connection();

    let _ = sql!(TableDeleteExpr.create());

    let id = sql!(TableDeleteExpr.insert(field1 = "", field2 = 0)).unwrap();

    let table = sql!(TableDeleteExpr.get(id));
    assert!(table.is_some());

    //assert_eq!(
        //"DELETE FROM TableDeleteExpr",
        //to_sql!(Table.delete()) // TODO: this does not work because the errors (including
        //warnings) return a dummy result.
    //);

    let num_deleted = sql!(TableDeleteExpr.filter(field1 == "").delete()).unwrap();
    assert_eq!(1, num_deleted);

    let table = sql!(TableDeleteExpr.get(id));
    assert!(table.is_none());

    let id1 = sql!(TableDeleteExpr.insert(field1 = "", field2 = 1)).unwrap();
    let id2 = sql!(TableDeleteExpr.insert(field1 = "", field2 = 2)).unwrap();
    let id3 = sql!(TableDeleteExpr.insert(field1 = "", field2 = 3)).unwrap();

    let table = sql!(TableDeleteExpr.get(id1));
    assert!(table.is_some());

    let table = sql!(TableDeleteExpr.get(id2));
    assert!(table.is_some());

    let table = sql!(TableDeleteExpr.get(id3));
    assert!(table.is_some());

    let num_deleted = sql!(TableDeleteExpr.filter(field2 < 5).delete()).unwrap();
    assert_eq!(3, num_deleted);

    let table = sql!(TableDeleteExpr.get(id1));
    assert!(table.is_none());

    let table = sql!(TableDeleteExpr.get(id2));
    assert!(table.is_none());

    let table = sql!(TableDeleteExpr.get(id3));
    assert!(table.is_none());

    let id = sql!(TableDeleteExpr.insert(field1 = "", field2 = 1)).unwrap();

    let num_deleted = sql!(TableDeleteExpr.filter(field2 > 5).delete()).unwrap();
    assert_eq!(0, num_deleted);

    let table = sql!(TableDeleteExpr.get(id));
    assert!(table.is_some());

    let num_deleted = sql!(TableDeleteExpr.get(id).delete()).unwrap();
    assert_eq!(1, num_deleted);

    let table = sql!(TableDeleteExpr.get(id));
    assert!(table.is_none());

    let _ = sql!(TableDeleteExpr.drop());
}
