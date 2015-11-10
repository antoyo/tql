#![feature(plugin)]
#![plugin(tql_macros)]

extern crate chrono;
extern crate postgres;
extern crate tql;

use chrono::datetime::DateTime;
use chrono::offset::utc::UTC;
use tql::{ForeignKey, PrimaryKey};

#[SqlTable]
#[allow(dead_code)]
#[derive(Debug)]
struct Table {
    id: PrimaryKey,
    field1: String,
    field2: i32,
    field3: Option<i32>,
    related_field: ForeignKey<RelatedTable>,
    date: DateTime<UTC>,
}

#[SqlTable]
#[allow(dead_code)]
#[derive(Debug)]
struct RelatedTable {
    id: PrimaryKey,
    field1: String,
}

#[test]
fn test_select() {
    let select = "SELECT Table.date, Table.field1, Table.field2, Table.field3, Table.id";
    assert_eq!(format!("{} FROM Table", select), to_sql!(Table.all()));
    assert_eq!(format!("{} FROM Table WHERE field1 = 'value1'", select), to_sql!(Table.filter(field1 == "value1")));
    assert_eq!(format!("{} FROM Table WHERE field1 = 'value1' AND field2 < 100 ORDER BY field2 DESC", select), to_sql!(Table.filter(field1 == "value1" && field2 < 100).sort(-field2)));
    assert_eq!(format!("{} FROM Table WHERE field2 < 100 AND field1 = 'value1' ORDER BY field2 DESC", select), to_sql!(Table.filter(field2 < 100 && field1 == "value1").sort(-field2)));
    assert_eq!(format!("{} FROM Table WHERE field2 >= 42 ORDER BY field1", select), to_sql!(Table.filter(field2 >= 42).sort(field1)));
    assert_eq!(format!("{} FROM Table WHERE field2 >= 42 OR field1 = 'te''\"\\st'", select), to_sql!(Table.filter(field2 >= 42 || field1 == "te'\"\\st")));
    assert_eq!(format!("{} FROM Table LIMIT 2", select), to_sql!(Table.all()[..2]));
    assert_eq!(format!("{} FROM Table OFFSET 1 LIMIT 2", select), to_sql!(Table[1..3]));
    assert_eq!(format!("{} FROM Table OFFSET 2 LIMIT 1", select), to_sql!(Table.all()[2]));
    assert_eq!(format!("{} FROM Table OFFSET 42 LIMIT 1", select), to_sql!(Table.all()[42]));
    assert_eq!(format!("{} FROM Table OFFSET 1 LIMIT 1", select), to_sql!(Table.all()[2 - 1]));
    assert_eq!(format!("{} FROM Table LIMIT 1", select), to_sql!(Table.all()[..2 - 1]));
    assert_eq!(format!("{} FROM Table OFFSET 1", select), to_sql!(Table.all()[2 - 1..]));
    assert_eq!(format!("{} FROM Table OFFSET $1 LIMIT 1", select), to_sql!(Table.all()[index]));
    assert_eq!(format!("{} FROM Table OFFSET $1 LIMIT $2", select), to_sql!(Table.all()[index..end_index]));
    assert_eq!(format!("{} FROM Table OFFSET $1 LIMIT 1", select), to_sql!(Table.all()[result()]));
    assert_eq!(format!("{} FROM Table OFFSET $1 LIMIT 1", select), to_sql!(Table.all()[strct.result()]));
    assert_eq!(format!("{} FROM Table OFFSET $1 LIMIT 1", select), to_sql!(Table.all()[index + 1]));
    assert_eq!(format!("{} FROM Table OFFSET $1 LIMIT 1", select), to_sql!(Table.all()[-index]));
    assert_eq!(format!("{} FROM Table OFFSET $1 LIMIT 1", select), to_sql!(Table.all()[-index as i64]));
    assert_eq!(format!("{} FROM Table WHERE field1 = $1", select), to_sql!(Table.filter(field1 == value1)));
    assert_eq!(format!("{} FROM Table WHERE field1 > $1", select), to_sql!(Table.filter(field1 > value1)));
    assert_eq!(format!("{} FROM Table WHERE field1 > $1 AND field2 = $2", select), to_sql!(Table.filter(field1 > value1 && field2 == value2)));
    assert_eq!(format!("{}, RelatedTable.field1, RelatedTable.id FROM Table INNER JOIN RelatedTable ON Table.related_field = RelatedTable.id", select), to_sql!(Table.join(related_field)));
    assert_eq!(format!("{} FROM Table WHERE related_field = $1", select), to_sql!(Table.filter(related_field == value1)));
    assert_eq!(format!("{} FROM Table WHERE id = 1", select), to_sql!(Table.get(1)));
    assert_eq!(format!("{} FROM Table WHERE id = $1", select), to_sql!(Table.get(id)));
    assert_eq!(format!("{} FROM Table WHERE field2 = 24 OFFSET 0 LIMIT 1", select), to_sql!(Table.get(field2 == 24))); // TODO: remove the "OFFSET 0" in the optimizer.
    assert_eq!(format!("{} FROM Table WHERE field1 = 'test' AND field2 = 24 OFFSET 0 LIMIT 1", select), to_sql!(Table.get(field1 == "test" && field2 == 24)));
    assert_eq!(format!("{} FROM Table WHERE field2 > 10 ORDER BY field2 OFFSET 1 LIMIT 2", select), to_sql!(Table.filter(field2 > 10).sort(field2)[1..3]));
    assert_eq!(format!("{} FROM Table WHERE (field1 = 'test' AND field2 = 24) OFFSET 0 LIMIT 1", select), to_sql!(Table.get((field1 == "test" && field2 == 24))));
    assert_eq!(format!("{} FROM Table WHERE NOT (field1 = 'test' AND field2 = 24) OFFSET 0 LIMIT 1", select), to_sql!(Table.get(!(field1 == "test" && field2 == 24))));
    assert_eq!(format!("{} FROM Table WHERE NOT (field2 < 24) OFFSET 0 LIMIT 1", select), to_sql!(Table.get(!(field2 < 24))));
    assert_eq!(format!("{} FROM Table WHERE field1 = 'value2' OR field2 < 100 AND field1 = 'value1'", select), to_sql!(Table.filter(field1 == "value2" || field2 < 100 && field1 == "value1")));
    assert_eq!(format!("{} FROM Table WHERE (field1 = 'value2' OR field2 < 100) AND field1 = 'value1'", select), to_sql!(Table.filter((field1 == "value2" || field2 < 100) && field1 == "value1")));
    assert_eq!(format!("{} FROM Table WHERE field3 IS NOT NULL", select), to_sql!(Table.filter(field3.is_some())));
    assert_eq!(format!("{} FROM Table WHERE field3 IS NULL", select), to_sql!(Table.filter(field3.is_none())));
    assert_eq!(format!("{} FROM Table WHERE EXTRACT(YEAR FROM date) = 2015", select), to_sql!(Table.filter(date.year() == 2015)));
    assert_eq!(format!("{} FROM Table WHERE EXTRACT(YEAR FROM date) = 2015 AND EXTRACT(MONTH FROM date) = 10 AND EXTRACT(DAY FROM date) = 26 AND EXTRACT(HOUR FROM date) = 1 AND EXTRACT(MINUTE FROM date) = 39 AND EXTRACT(SECOND FROM date) > 0", select), to_sql!(Table.filter(date.year() == 2015 && date.month() == 10 && date.day() == 26 && date.hour() == 1 && date.minute() == 39 && date.second() > 0)));
    assert_eq!(format!("{} FROM Table WHERE field1 LIKE '%' || 'value' || '%' = TRUE", select), to_sql!(Table.filter(field1.contains("value") == true)));
    assert_eq!(format!("{} FROM Table WHERE field1 LIKE '%' || 'value' || '%'", select), to_sql!(Table.filter(field1.contains("value"))));
    assert_eq!(format!("{} FROM Table WHERE field1 LIKE 'va' || '%'", select), to_sql!(Table.filter(field1.starts_with("va"))));
    assert_eq!(format!("{} FROM Table WHERE field1 LIKE '%' || 'e1'", select), to_sql!(Table.filter(field1.ends_with("e1"))));
    assert_eq!(format!("{} FROM Table WHERE field1 LIKE '%' || $1 || '%'", select), to_sql!(Table.filter(field1.contains(value))));
    assert_eq!(format!("{} FROM Table WHERE CHAR_LENGTH(field1) = 6", select), to_sql!(Table.filter(field1.len() == 6)));
    assert_eq!(format!("{} FROM Table WHERE field1 LIKE '%3'", select), to_sql!(Table.filter(field1.match(r"%3"))));
    assert_eq!(format!("{} FROM Table WHERE field1 LIKE '%E3'", select), to_sql!(Table.filter(field1.match(r"%E3"))));
    assert_eq!(format!("{} FROM Table WHERE field1 ILIKE '%E3'", select), to_sql!(Table.filter(field1.imatch(r"%E3"))));
    assert_eq!(format!("{} FROM Table WHERE id = 2 OFFSET 0 LIMIT 1", select), to_sql!(Table.filter(id == 2).get()));
}
