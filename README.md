| SQL                   | Rust                      |
|:----------------------|:--------------------------|
| <pre lang="sql">SELECT * FROM Table</pre> | <pre lang="rust">Table.all()</pre> |
| SELECT field1 FROM Table | Table.only(field1) |
| SELECT field1 FROM Table | Table.defer(pk) // Exclusion de champs. |
| SELECT * FROM Table WHERE field1 = "value1" | Table.filter(field1 == "value1") |
| SELECT * FROM Table WHERE primary_key = 42 | <code>Table.get(42)<br/>// Raccourci pour :<br/>Table.filter(primary_key == 42)[0..1];</code> |
| SELECT * FROM Table WHERE field1 = 'value1' | <code>Table.get(field1 == "value1")<br/>// Raccourci pour :<br/>Table.filter(field1 == "value1")[0..1];</code> |
| SELECT * FROM Table WHERE field1 = "value1 AND field2 < 100 | Table.filter(field1 == "value1" && field2 < 100) |
| SELECT * FROM Table WHERE field1 = "value1 OR field2 < 100 | Table.filter(field1 == "value1" || field2 < 100) |
| SELECT * FROM Table ORDER BY field1 | Table.sort(field1) |
| SELECT * FROM Table ORDER BY field1 DESC | Table.sort(-field1) |
| SELECT * FROM Table LIMIT 0, 20 | Table[0..20] |
| SELECT * FROM Table WHERE field1 = "value1" AND field2 < 100 ORDER BY field2 DESC LIMIT 10, 20 | Table.filter(field1 == "value1" && field2 < 100).sort(-field2)[10..20] |
| INSERT INTO Table(field1, field2) VALUES("value1", 55) | <code>let table = Table {<br/>    field1: "value1",<br/>    field2: 55,<br/>};<br/> table.insert()</code> |
| UPDATE Table SET field1 = "value1", field2 = 55 WHERE id = 1 | <code>Table.get(1).update(field1 = "value1", field2 = 55);<br/>// ou<br/>Table.filter(id == 1).update(field1 = "value1", field2 = 55);<br/>// ou<br/>let table = Table.get(1);<br/>table.field1 = "value1";<br/>table.field2 = 55;<br/>table.update();</code> |
| DELETE FROM Table WHERE id = 1 | <code>Table.get(1).delete();<br/>// ou<br/>Table.filter(id == 1).delete()</code> |
| SELECT AVG(field2) FROM Table | Table.aggregate(avg(field2)) |
| SELECT AVG(field1) FROM Table1 GROUP BY field2 | Table1.values(field2).annotate(avg(field1)) |
| SELECT AVG(field1) as average FROM Table1 GROUP BY field2 HAVING average > 5 | Table1.values(field2).annotate(average = avg(field1)).filter(average > 5) |
| SELECT AVG(field1) as average FROM Table1 WHERE field1 < 10 GROUP BY field2 HAVING average > 5 | Table1.filter(field1 < 10).values(field2).annotate(average = avg(field1)).filter(average > 5) |
| SELECT Table1.field1, Table2.field1 FROM Table1 INNER JOIN Table2 ON Table1.pk = Table2.fk | <code>#[sql_table]<br/>struct Table1 {<br/>    pk: db::PrimaryKey,<br/>    field1: i32,<br/>}<br/>#[sql_table]<br/>struct Table2 {<br/>    field1: i32,<br/>    fk: db::ForeignKey<Table1>,<br/>}<br/>Table1.all().join(Table2)</code> |
| SELECT * FROM Table1 WHERE YEAR(date) = 2015 | Table1.filter(date.year() == 2015) |
| SELECT * FROM Table1 WHERE INSTR(field1, 'string') > 0 | Table1.filter(field1.contains("string")) |
| SELECT * FROM Table1 WHERE field1 in (1, 2, 3) | Table1.filter([1, 2, 3].contains(field1)) |
| SELECT * FROM Table1 WHERE field1 LIKE 'string%' | Table1.filter(field1.starts_with("string")) |
| SELECT * FROM Table1 WHERE field1 LIKE '%string' | Table1.filter(field1.ends_with("string")) |
| SELECT * FROM Table1 WHERE field1 BETWEEN 1 AND 5 | Table1.filter(field1 in 1..6) |
| SELECT * FROM Table1 WHERE field1 IS NULL | Table1.filter(field1.is_none()) |
| SELECT * FROM Table1 WHERE field1 REGEXP BINARY '\^[a-d]' | Table1.filter(r"^[a-d]".is_match(field1)) |
| SELECT * FROM Table1 WHERE field1 REGEXP '^[a-d]' | Table1.filter(r"^[a-d]".is_match(field1, db::CaseInsensitive)) |
| <code>CREATE TABLE IF NOT EXISTS Table1 (<br/>    pk INTEGER NOT NULL AUTO_INCREMENT,<br/>    field1 INTEGER,<br/>    PRIMARY KEY (pk)<br/>)</code> | <code>#[sql_table]<br/>struct Table1 {<br/>    pk: db::PrimaryKey,<br/>    field1: i32,<br/>}<br/>Table1.create()</code |
