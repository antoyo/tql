/*
 * Copyright (C) 2015  Boucher, Antoni <bouanto@zoho.com>
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

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
struct RelatedTable {
    id: PrimaryKey,
    field1: String,
}

const SELECT: &'static str = "SELECT Table.date, Table.field1, Table.field2, Table.field3, Table.id";

#[test]
fn test_all() {
    assert_eq!(
        format!("{} FROM Table", SELECT),
        to_sql!(Table.all())
    );
}

#[test]
fn test_filter() {
    assert_eq!(
        format!("{} FROM Table WHERE field1 = 'value1'", SELECT),
        to_sql!(Table.filter(field1 == "value1"))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field2 >= 42 OR field1 = 'te''\"\\st'", SELECT),
        to_sql!(Table.filter(field2 >= 42 || field1 == "te'\"\\st"))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field1 = $1", SELECT),
        to_sql!(Table.filter(field1 == value1))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field1 > $1", SELECT),
        to_sql!(Table.filter(field1 > value1))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field1 > $1 AND field2 = $2", SELECT),
        to_sql!(Table.filter(field1 > value1 && field2 == value2))
    );
    assert_eq!(
        format!("{} FROM Table WHERE related_field = $1", SELECT),
        to_sql!(Table.filter(related_field == value1))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field1 = 'value2' OR field2 < 100 AND field1 = 'value1'", SELECT),
        to_sql!(Table.filter(field1 == "value2" || field2 < 100 && field1 == "value1"))
    );
    assert_eq!(
        format!("{} FROM Table WHERE (field1 = 'value2' OR field2 < 100) AND field1 = 'value1'", SELECT),
        to_sql!(Table.filter((field1 == "value2" || field2 < 100) && field1 == "value1"))
    );
    assert_eq!(
        format!("{} FROM Table WHERE (field1 = 'test' AND field2 = 24)", SELECT),
        to_sql!(Table.filter((field1 == "test" && field2 == 24)))
    );
    assert_eq!(
        format!("{} FROM Table WHERE NOT (field1 = 'test' AND field2 = 24)", SELECT),
        to_sql!(Table.filter(!(field1 == "test" && field2 == 24)))
    );
    assert_eq!(
        format!("{} FROM Table WHERE NOT (field2 < 24)", SELECT),
        to_sql!(Table.filter(!(field2 < 24)))
    );
}

#[test]
fn test_filter_method_call() {
    assert_eq!(
        format!("{} FROM Table WHERE field3 IS NOT NULL", SELECT),
        to_sql!(Table.filter(field3.is_some()))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field3 IS NULL", SELECT),
        to_sql!(Table.filter(field3.is_none()))
    );
    assert_eq!(
        format!("{} FROM Table WHERE EXTRACT(YEAR FROM date) = 2015", SELECT),
        to_sql!(Table.filter(date.year() == 2015))
    );
    assert_eq!(
        format!("{} FROM Table WHERE EXTRACT(YEAR FROM date) = 2015 AND EXTRACT(MONTH FROM date) = 10 AND EXTRACT(DAY FROM date) = 26 AND EXTRACT(HOUR FROM date) = 1 AND EXTRACT(MINUTE FROM date) = 39 AND EXTRACT(SECOND FROM date) > 0", SELECT),
        to_sql!(Table.filter(date.year() == 2015 && date.month() == 10 && date.day() == 26 && date.hour() == 1 && date.minute() == 39 && date.second() > 0))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field1 LIKE '%' || 'value' || '%' = TRUE", SELECT),
        to_sql!(Table.filter(field1.contains("value") == true))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field1 LIKE '%' || 'value' || '%'", SELECT),
        to_sql!(Table.filter(field1.contains("value")))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field1 LIKE 'va' || '%'", SELECT),
        to_sql!(Table.filter(field1.starts_with("va")))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field1 LIKE '%' || 'e1'", SELECT),
        to_sql!(Table.filter(field1.ends_with("e1")))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field1 LIKE '%' || $1 || '%'", SELECT),
        to_sql!(Table.filter(field1.contains(value)))
    );
    assert_eq!(
        format!("{} FROM Table WHERE CHAR_LENGTH(field1) = 6", SELECT),
        to_sql!(Table.filter(field1.len() == 6))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field1 LIKE '%3'", SELECT),
        to_sql!(Table.filter(field1.match(r"%3")))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field1 LIKE '%E3'", SELECT),
        to_sql!(Table.filter(field1.match(r"%E3")))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field1 ILIKE '%E3'", SELECT),
        to_sql!(Table.filter(field1.imatch(r"%E3")))
    );
}

#[test]
fn test_filter_get() {
    assert_eq!(
        format!("{} FROM Table WHERE id = 2 OFFSET 0 LIMIT 1", SELECT),
        to_sql!(Table.filter(id == 2).get())
    );
}

#[test]
fn test_filter_sort() {
    assert_eq!(
        format!("{} FROM Table WHERE field1 = 'value1' AND field2 < 100 ORDER BY field2 DESC", SELECT),
        to_sql!(Table.filter(field1 == "value1" && field2 < 100).sort(-field2))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field2 < 100 AND field1 = 'value1' ORDER BY field2 DESC", SELECT),
        to_sql!(Table.filter(field2 < 100 && field1 == "value1").sort(-field2))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field2 >= 42 ORDER BY field1", SELECT),
        to_sql!(Table.filter(field2 >= 42).sort(field1))
    );
}

#[test]
fn test_filter_sort_limit() {
    assert_eq!(
        format!("{} FROM Table WHERE field2 > 10 ORDER BY field2 OFFSET 1 LIMIT 2", SELECT),
        to_sql!(Table.filter(field2 > 10).sort(field2)[1..3])
    );
}

#[test]
fn test_get() {
    assert_eq!(
        format!("{} FROM Table WHERE id = 1", SELECT),
        to_sql!(Table.get(1))
    );
    assert_eq!(
        format!("{} FROM Table WHERE id = $1", SELECT),
        to_sql!(Table.get(id))
    );
    assert_eq!(
        format!("{} FROM Table WHERE field2 = 24 OFFSET 0 LIMIT 1", SELECT),
        to_sql!(Table.get(field2 == 24))
    ); // TODO: remove the "OFFSET 0" in the optimizer.
    assert_eq!(
        format!("{} FROM Table WHERE field1 = 'test' AND field2 = 24 OFFSET 0 LIMIT 1", SELECT),
        to_sql!(Table.get(field1 == "test" && field2 == 24))
    );
    assert_eq!(
        format!("{} FROM Table WHERE (field1 = 'test' AND field2 = 24) OFFSET 0 LIMIT 1", SELECT),
        to_sql!(Table.get((field1 == "test" && field2 == 24)))
    );
    assert_eq!(
        format!("{} FROM Table WHERE NOT (field1 = 'test' AND field2 = 24) OFFSET 0 LIMIT 1", SELECT),
        to_sql!(Table.get(!(field1 == "test" && field2 == 24)))
    );
    assert_eq!(
        format!("{} FROM Table WHERE NOT (field2 < 24) OFFSET 0 LIMIT 1", SELECT),
        to_sql!(Table.get(!(field2 < 24)))
    );
}

#[test]
fn test_join() {
    assert_eq!(
        format!("{}, RelatedTable.field1, RelatedTable.id FROM Table INNER JOIN RelatedTable ON Table.related_field = RelatedTable.id", SELECT),
        to_sql!(Table.join(related_field))
    );
    assert_eq!(
        format!("{}, RelatedTable.field1, RelatedTable.id FROM Table INNER JOIN RelatedTable ON Table.related_field = RelatedTable.id", SELECT),
        to_sql!(Table.all().join(related_field))
    );
}

#[test]
fn test_limit() {
    assert_eq!(
        format!("{} FROM Table LIMIT 2", SELECT),
        to_sql!(Table.all()[..2])
    );
    assert_eq!(
        format!("{} FROM Table LIMIT 2", SELECT),
        to_sql!(Table[..2])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET 1 LIMIT 2", SELECT),
        to_sql!(Table[1..3])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET 2 LIMIT 1", SELECT),
        to_sql!(Table.all()[2])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET 2 LIMIT 1", SELECT),
        to_sql!(Table[2])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET 42 LIMIT 1", SELECT),
        to_sql!(Table.all()[42])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET 1 LIMIT 1", SELECT),
        to_sql!(Table.all()[2 - 1])
    );
    assert_eq!(
        format!("{} FROM Table LIMIT 1", SELECT),
        to_sql!(Table.all()[..2 - 1])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET 1", SELECT),
        to_sql!(Table.all()[2 - 1..])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET 3", SELECT),
        to_sql!(Table.all()[2 + 1..])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET 2", SELECT),
        to_sql!(Table.all()[2 + 1 - 3 + 2..])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET $1 LIMIT 1", SELECT),
        to_sql!(Table.all()[index])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET $1 LIMIT $2", SELECT),
        to_sql!(Table.all()[index..end_index])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET $1 LIMIT 1", SELECT),
        to_sql!(Table.all()[result()])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET $1 LIMIT 1", SELECT),
        to_sql!(Table.all()[strct.result()])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET $1 LIMIT 1", SELECT),
        to_sql!(Table.all()[index + 1])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET $1 LIMIT 1", SELECT),
        to_sql!(Table.all()[-index])
    );
    assert_eq!(
        format!("{} FROM Table OFFSET $1 LIMIT 1", SELECT),
        to_sql!(Table.all()[-index as i64])
    );
}
