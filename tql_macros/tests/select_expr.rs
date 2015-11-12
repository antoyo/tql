#![feature(box_patterns, plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use postgres::{Connection, SslMode};
use tql::{ForeignKey, PrimaryKey};

#[SqlTable]
#[derive(Debug)]
struct TableSelectExpr {
    id: PrimaryKey,
    field1: String,
    field2: i32,
    related_field: ForeignKey<RelatedTableSelectExpr>,
}

#[SqlTable]
#[derive(Debug)]
struct RelatedTableSelectExpr {
    id: PrimaryKey,
    field1: i32,
}

fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", &SslMode::None).unwrap()
}

#[test]
fn test_select() {
    let connection = get_connection();

    let _ = sql!(TableSelectExpr.create());
    let _ = sql!(RelatedTableSelectExpr.create());

    let id = sql!(RelatedTableSelectExpr.insert(field1 = 42)).unwrap();
    let related_field = sql!(RelatedTableSelectExpr.get(id)).unwrap();
    let id = sql!(RelatedTableSelectExpr.insert(field1 = 24)).unwrap();
    let related_field2 = sql!(RelatedTableSelectExpr.get(id)).unwrap();
    let id1 = sql!(TableSelectExpr.insert(field1 = "value1", field2 = 55, related_field = related_field)).unwrap();
    let new_field2 = 42;
    let id2 = sql!(TableSelectExpr.insert(field1 = "value2", field2 = new_field2, related_field = related_field)).unwrap();
    let id3 = sql!(TableSelectExpr.insert(field1 = "value3", field2 = 12, related_field = related_field2)).unwrap();

    let tables = sql!(TableSelectExpr.all());
    let table1 = &tables[0];
    let table2 = &tables[1];
    let table3 = &tables[2];
    assert_eq!(3, tables.len());
    assert_eq!(id1, table1.id);
    assert_eq!("value1", table1.field1);
    assert_eq!(55, table1.field2);
    assert_eq!(id2, table2.id);
    assert_eq!("value2", table2.field1);
    assert_eq!(42, table2.field2);
    assert_eq!(id3, table3.id);
    assert_eq!("value3", table3.field1);
    assert_eq!(12, table3.field2);

    let tables = sql!(TableSelectExpr.filter(field1 == "value1"));
    let table = &tables[0];
    assert_eq!(1, tables.len());
    assert_eq!("value1", table.field1);
    assert_eq!(55, table.field2);

    let tables = sql!(TableSelectExpr.filter(field2 >= 42 || field1 == "te'\"\\st"));
    let table1 = &tables[0];
    let table2 = &tables[1];
    assert_eq!(2, tables.len());
    assert_eq!("value1", table1.field1);
    assert_eq!(55, table1.field2);
    assert_eq!("value2", table2.field1);
    assert_eq!(42, table2.field2);

    let value = 42;
    let tables = sql!(TableSelectExpr.filter(field2 == value));
    let table = &tables[0];
    assert_eq!(1, tables.len());
    assert_eq!("value2", table.field1);
    assert_eq!(42, table.field2);
    
    let tables = sql!(TableSelectExpr.filter(field2 > value));
    let table = &tables[0];
    assert_eq!(1, tables.len());
    assert_eq!("value1", table.field1);
    assert_eq!(55, table.field2);

    let value2 = "value1";
    let tables = sql!(TableSelectExpr.filter(field2 > value && field1 == value2));
    let table = &tables[0];
    assert_eq!(1, tables.len());
    assert_eq!("value1", table.field1);
    assert_eq!(55, table.field2);

    let value2 = "value2";
    let tables = sql!(TableSelectExpr.filter(field2 > value && field1 == value2));
    assert_eq!(0, tables.len());

    let tables = sql!(TableSelectExpr.filter(related_field == related_field));
    let table1 = &tables[0];
    let table2 = &tables[1];
    assert_eq!(2, tables.len());
    assert_eq!(id1, table1.id);
    assert_eq!(id2, table2.id);

    let tables = sql!(TableSelectExpr.filter(related_field == related_field2));
    let table1 = &tables[0];
    assert_eq!(1, tables.len());
    assert_eq!(id3, table1.id);

    let tables = sql!(TableSelectExpr.filter(field1 == "value2" || field2 < 100 && field1 == "value1"));
    let table1 = &tables[0];
    let table2 = &tables[1];
    assert_eq!(2, tables.len());
    assert_eq!(id1, table1.id);
    assert_eq!(id2, table2.id);

    let tables = sql!(TableSelectExpr.filter((field1 == "value2" || field2 < 100) && field1 == "value1"));
    let table1 = &tables[0];
    assert_eq!(1, tables.len());
    assert_eq!(id1, table1.id);

    let tables = sql!(TableSelectExpr.filter((field1 == "value3" && field2 == 12)));
    let table1 = &tables[0];
    assert_eq!(1, tables.len());
    assert_eq!(id3, table1.id);

    let tables = sql!(TableSelectExpr.filter(!(field1 == "value3" && field2 == 12)));
    let table1 = &tables[0];
    let table2 = &tables[1];
    assert_eq!(2, tables.len());
    assert_eq!(id1, table1.id);
    assert_eq!(id2, table2.id);

    let tables = sql!(TableSelectExpr.filter(!(field2 < 24)));
    assert_eq!(2, tables.len());
    assert_eq!(id1, table1.id);
    assert_eq!(id2, table2.id);

    let _ = sql!(RelatedTableSelectExpr.drop());
    let _ = sql!(TableSelectExpr.drop());
}
