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
fn test_aggregate() {
    assert_eq!(
        "SELECT CAST(AVG(field2) AS INT) FROM Table",
        to_sql!(Table.aggregate(avg(field2)))
    );
    assert_eq!(
        "SELECT CAST(AVG(field2) AS INT) FROM Table GROUP BY field1",
        to_sql!(Table.values(field1).aggregate(avg(field2)))
    );
    assert_eq!(
        "SELECT CAST(AVG(field2) AS INT) FROM Table",
        to_sql!(Table.aggregate(average = avg(field2)))
    );
    assert_eq!(
        "SELECT CAST(AVG(field2) AS INT) FROM Table GROUP BY field1 HAVING CAST(AVG(field2) AS INT) < 20",
        to_sql!(Table.values(field1).aggregate(average = avg(field2)).filter(average < 20))
    );
    assert_eq!(
        "SELECT CAST(AVG(field2) AS INT) FROM Table GROUP BY field1 HAVING CAST(AVG(field2) AS INT) < 20",
        to_sql!(Table.values(field1).aggregate(avg(field2)).filter(field2_avg < 20))
    );
    assert_eq!(
        "SELECT CAST(AVG(field2) AS INT) FROM Table WHERE field2 > 10 GROUP BY field1 HAVING CAST(AVG(field2) AS INT) < 20",
        to_sql!(Table.filter(field2 > 10).values(field1).aggregate(avg(field2)).filter(field2_avg < 20))
    );
    assert_eq!(
        "SELECT CAST(AVG(field2) AS INT) FROM Table WHERE field2 > 10 GROUP BY field1 HAVING CAST(AVG(field2) AS INT) < 20",
        to_sql!(Table.filter(field2 > 10).values(field1).aggregate(average = avg(field2)).filter(average < 20))
    );
}
