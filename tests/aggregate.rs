#![feature(proc_macro)]

extern crate postgres;
extern crate tql;
#[macro_use]
extern crate tql_macros;

use tql::PrimaryKey;
use tql_macros::to_sql;

#[derive(SqlTable)]
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
        "SELECT CAST(AVG(field2) AS INT) FROM Table WHERE Table.field2 > 10 GROUP BY field1 HAVING CAST(AVG(field2) AS INT) < 20",
        to_sql!(Table.filter(field2 > 10).values(field1).aggregate(avg(field2)).filter(field2_avg < 20))
    );
    assert_eq!(
        "SELECT CAST(AVG(field2) AS INT) FROM Table WHERE Table.field2 > 10 GROUP BY field1 HAVING CAST(AVG(field2) AS INT) < 20",
        to_sql!(Table.filter(field2 > 10).values(field1).aggregate(average = avg(field2)).filter(average < 20))
    );
}
