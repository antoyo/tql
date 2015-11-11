#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use tql::PrimaryKey;

#[SqlTable]
#[allow(dead_code)]
#[derive(Debug)]
struct Table {
    id: PrimaryKey,
    field1: String,
    field2: i32,
}

#[test]
fn test_delete() {
    //assert_eq!(
        //"DELETE FROM Table",
        //to_sql!(Table.delete()) // TODO: ceci ne fonctionne pas. Le problème vient du fait que
        //les erreurs (incluant les avertissements) retourne un résultat bidon.
    //);
    assert_eq!(
        "DELETE FROM Table WHERE field1 = 'test'",
        to_sql!(Table.filter(field1 == "test").delete())
    );
    // TODO: Dans tests/delete_expr.rs, vérifier que le nombre retourné par delete() est correct.
}
