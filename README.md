| SQL                   | Rust                      |
|:----------------------|:--------------------------|
| <pre>SELECT * FROM Table</pre> | <pre>Table.all()</pre> |
| <pre>SELECT field1 FROM Table</pre> | <pre>Table.only(field1)</pre> |
| <pre>SELECT field1 FROM Table</pre> | <pre>Table.defer(pk) // Exclusion de champs.</pre> |
| <pre>SELECT * FROM Table WHERE field1 = "value1"</pre> | <pre>Table.filter(field1 == "value1")</pre> |
| <pre>SELECT * FROM Table WHERE primary_key = 42</pre> | <pre>Table.get(42)<br/>// Raccourci pour :<br/>Table.filter(primary_key == 42)[0..1];</pre> |
| <pre>SELECT * FROM Table WHERE field1 = 'value1'</pre> | <pre>Table.get(field1 == "value1")<br/>// Raccourci pour :<br/>Table.filter(field1 == "value1")[0..1];</pre> |
| <pre>SELECT * FROM Table WHERE field1 = "value1 AND field2 < 100</pre> | <pre>Table.filter(field1 == "value1" && field2 < 100)</pre> |
| <pre>SELECT * FROM Table WHERE field1 = "value1 OR field2 < 100</pre> | <pre>Table.filter(field1 == "value1" || field2 < 100)</pre> |
| <pre>SELECT * FROM Table ORDER BY field1</pre> | <pre>Table.sort(field1)</pre> |
| <pre>SELECT * FROM Table ORDER BY field1 DESC</pre> | <pre>Table.sort(-field1)</pre> |
| <pre>SELECT * FROM Table LIMIT 0, 20</pre> | <pre>Table[0..20]</pre> |
| <pre>SELECT * FROM Table<br/>WHERE field1 = "value1"<br/>    AND field2 < 100<br/>ORDER BY field2 DESC<br/>LIMIT 10, 20</pre> | <pre>Table.filter(field1 == "value1" && field2 < 100).sort(-field2)[10..20]</pre> |
| <pre>INSERT INTO Table(field1, field2) VALUES("value1", 55)</pre> | let table = <pre>Table {<br/>    field1: "value1",<br/>    field2: 55,<br/>};<br/> table.insert()</pre> |
| <pre>UPDATE Table SET field1 = "value1", field2 = 55 WHERE id = 1</pre> | <pre>Table.get(1).update(field1 = "value1", field2 = 55);<br/>// ou<br/>Table.filter(id == 1).update(field1 = "value1", field2 = 55);<br/>// ou<br/>let table = Table.get(1);<br/>table.field1 = "value1";<br/>table.field2 = 55;<br/>table.update();</pre> |
| <pre>DELETE FROM Table WHERE id = 1</pre> | <pre>Table.get(1).delete();<br/>// ou<br/>Table.filter(id == 1).delete()</pre> |
| <pre>SELECT AVG(field2) FROM Table</pre> | <pre>Table.aggregate(avg(field2))</pre> |
| <pre>SELECT AVG(field1) FROM Table1 GROUP BY field2</pre> | <pre>Table1.values(field2).annotate(avg(field1))</pre> |
| <pre>SELECT AVG(field1) as average FROM Table1<br/>GROUP BY field2<br/>HAVING average > 5</pre> | <pre>Table1.values(field2).annotate(average = avg(field1)).filter(average > 5)</pre> |
| <pre>SELECT AVG(field1) as average FROM Table1<br/>WHERE field1 < 10<br/>GROUP BY field2<br/>HAVING average > 5</pre> | <pre>Table1.filter(field1 < 10).values(field2).annotate(average = avg(field1)).filter(average > 5)</pre> |
| <pre>SELECT Table1.field1, Table2.field1 FROM Table1<br/>INNER JOIN Table2 ON Table1.pk = Table2.fk</pre> | <pre>#[sql_table]<br/>struct Table1 {<br/>    pk: db::PrimaryKey,<br/>    field1: i32,<br/>}<br/>#[sql_table]<br/>struct Table2 {<br/>    field1: i32,<br/>    fk: db::ForeignKey<Table1>,<br/>}<br/>Table1.all().join(Table2)</pre> |
| <pre>SELECT * FROM Table1 WHERE YEAR(date) = 2015</pre> | <pre>Table1.filter(date.year() == 2015)</pre> |
| <pre>SELECT * FROM Table1 WHERE INSTR(field1, 'string') > 0</pre> | <pre>Table1.filter(field1.contains("string"))</pre> |
| <pre>SELECT * FROM Table1 WHERE field1 in (1, 2, 3)</pre> | <pre>Table1.filter([1, 2, 3].contains(field1))</pre> |
| <pre>SELECT * FROM Table1 WHERE field1 LIKE 'string%'</pre> | <pre>Table1.filter(field1.starts_with("string"))</pre> |
| <pre>SELECT * FROM Table1 WHERE field1 LIKE '%string'</pre> | <pre>Table1.filter(field1.ends_with("string"))</pre> |
| <pre>SELECT * FROM Table1 WHERE field1 BETWEEN 1 AND 5</pre> | <pre>Table1.filter(field1 in 1..6)</pre> |
| <pre>SELECT * FROM Table1 WHERE field1 IS NULL</pre> | <pre>Table1.filter(field1.is_none())</pre> |
| <pre>SELECT * FROM Table1 WHERE field1 REGEXP BINARY '\^[a-d]'</pre> | <pre>Table1.filter(r"\^[a-d]".is_match(field1))</pre> |
| <pre>SELECT * FROM Table1 WHERE field1 REGEXP '\^[a-d]'</pre> | <pre>Table1.filter(r"\^[a-d]".is_match(field1, db::CaseInsensitive))</pre> |
| <pre>CREATE TABLE IF NOT EXISTS Table1 (<br/>    pk INTEGER NOT NULL AUTO_INCREMENT,<br/>    field1 INTEGER,<br/>    PRIMARY KEY (pk)<br/>)</pre> | <pre>#[sql_table]<br/>struct Table1 {<br/>    pk: db::PrimaryKey,<br/>    field1: i32,<br/>}<br/>Table1.create()</pre> |