#![feature(plugin)]
#![plugin(tql_macros)]

extern crate postgres;
extern crate tql;

use tql::{ForeignKey, PrimaryKey};

#[SqlTable]
#[allow(dead_code)]
#[derive(Debug)]
struct Table {
    id: PrimaryKey,
    field1: String,
    field2: i32,
    related_field: ForeignKey<RelatedTable>,
}

#[SqlTable]
#[allow(dead_code)]
#[derive(Debug)]
struct RelatedTable {
    id: PrimaryKey,
    field1: String,
}

#[test]
fn test_update() {
    assert_eq!(
        "UPDATE Table SET field1 = 'value1', field2 = 55 WHERE id = 1",
        to_sql!(Table.get(1).update(field1 = "value1", field2 = 55))
    );
    assert_eq!(
        "UPDATE Table SET field1 = 'value1', field2 = $1 WHERE id = 1",
        to_sql!(Table.filter(id == 1).update(field1 = "value1", field2 = new_field2))
    );
}

#[test]
fn test_update_operation() {
    assert_eq!(
        "UPDATE Table SET field2 = field2 + 1 WHERE id = 1",
        to_sql!(Table.get(1).update(field2 += 1))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 - 3 WHERE id = 1",
        to_sql!(Table.get(1).update(field2 -= 3))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 % 7 WHERE id = 1",
        to_sql!(Table.get(1).update(field2 %= 7))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 * 2 WHERE id = 1",
        to_sql!(Table.get(1).update(field2 *= 2))
    );
    assert_eq!(
        "UPDATE Table SET field2 = field2 / 3 WHERE id = 1",
        to_sql!(Table.get(1).update(field2 /= 3))
    );
}
