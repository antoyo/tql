#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use postgres::{Connection, SslMode};
use tql::PrimaryKey;

#[SqlTable]
#[allow(dead_code)]
#[derive(Debug)]
struct SqlTable {
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

    let _ = sql!(SqlTable.create());

    let id = sql!(SqlTable.insert(field1 = "", field2 = 0)).unwrap();

    let table = sql!(SqlTable.get(id));
    assert!(table.is_some());

    //assert_eq!(
        //"DELETE FROM SqlTable",
        //to_sql!(SqlTable.delete()) // TODO: ceci ne fonctionne pas. Le problème vient du fait que
        //les erreurs (incluant les avertissements) retourne un résultat bidon.
    //);

    let num_deleted = sql!(SqlTable.filter(field1 == "").delete()).unwrap();
    assert_eq!(1, num_deleted);

    let table = sql!(SqlTable.get(id));
    assert!(table.is_none());

    let id1 = sql!(SqlTable.insert(field1 = "", field2 = 1)).unwrap();
    let id2 = sql!(SqlTable.insert(field1 = "", field2 = 2)).unwrap();
    let id3 = sql!(SqlTable.insert(field1 = "", field2 = 3)).unwrap();

    let table = sql!(SqlTable.get(id1));
    assert!(table.is_some());

    let table = sql!(SqlTable.get(id2));
    assert!(table.is_some());

    let table = sql!(SqlTable.get(id3));
    assert!(table.is_some());

    let num_deleted = sql!(SqlTable.filter(field2 < 5).delete()).unwrap();
    assert_eq!(3, num_deleted);

    let table = sql!(SqlTable.get(id1));
    assert!(table.is_none());

    let table = sql!(SqlTable.get(id2));
    assert!(table.is_none());

    let table = sql!(SqlTable.get(id3));
    assert!(table.is_none());

    let id = sql!(SqlTable.insert(field1 = "", field2 = 1)).unwrap();

    let num_deleted = sql!(SqlTable.filter(field2 > 5).delete()).unwrap();
    assert_eq!(0, num_deleted);

    let table = sql!(SqlTable.get(id));
    assert!(table.is_some());

    let num_deleted = sql!(SqlTable.get(id).delete()).unwrap();
    assert_eq!(1, num_deleted);

    let table = sql!(SqlTable.get(id));
    assert!(table.is_none());

    let _ = sql!(SqlTable.drop());
}
