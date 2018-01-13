/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! TQL is a Rust compiler plugin providing an SQL DSL.
//!
//! It type check your expression at compile time and converts it to SQL.

#[cfg(feature = "chrono")]
extern crate chrono;
#[cfg(feature = "postgres")]
extern crate postgres;

mod methods;
mod types;

use std::collections::HashMap;
use std::mem;
use std::sync::{Arc, Mutex, Once, ONCE_INIT};

#[cfg(feature = "postgres")]
use postgres::Connection;
#[cfg(feature = "postgres")]
use postgres::stmt::Column;
#[cfg(feature = "postgres")]
use postgres::types::Oid;

pub use types::{Date, DateTime, Time, ToTqlType};
pub use types::numbers::{i16, i32, i64, i8, u16, u32, u64, u8};

/// The `ForeignKey` is optional.
///
/// There is no value when the `join()` method is not called.
pub type ForeignKey<T> = Option<T>;

/// A `PrimaryKey` is a 4-byte integer.
pub type PrimaryKey = types::StdI32;

#[doc(hidden)]
// Marker trait used for error reporting:
// when a struct is used in a ForeignKey, but it is not annotated with #[derive(SqlTable)].
pub unsafe trait SqlTable {
    fn _create_query() -> &'static str;

    fn default() -> Self;

    #[cfg(feature = "postgres")]
    fn from_row(row: &::postgres::rows::Row, columns: &[::postgres::stmt::Column]) -> Self;
}

#[cfg(not(unstable))]
#[macro_export]
macro_rules! check_missing_fields {
    ($($tt:tt)*) => {{
        #[derive(StableCheckMissingFields)]
        enum __TqlStableCheckMissingFieldEnum {
            Input = (stringify!($($tt)*), 0).1,
        }

        __tql_call_macro_missing_fields!()
    }};
}

#[cfg(not(unstable))]
#[macro_export]
macro_rules! sql {
    ($($tt:tt)*) => {{
        #[derive(StableToSql)]
        enum __TqlStableToSqlEnum {
            Input = (stringify!($($tt)*), 0).1,
        }

        __tql_call_macro!()
    }};
}

#[cfg(feature = "postgres")]
#[doc(hidden)]
pub fn index_from_table_column(table: &str, column_name: &str, columns: &[Column]) -> usize {
    let table_state = table_singleton();
    if let Ok(table_state) = table_state.inner.lock() {
        if let Some(&table_oid) = table_state.get(table) {
            for (index, column) in columns.iter().enumerate() {
                if column.table() == table_oid && column.name() == column_name {
                    return index;
                }
            }
        }
    }
    panic!("Make sure you called tql::init() first");
}

#[cfg(feature = "postgres")]
#[derive(Clone)]
struct TableState {
    inner: Arc<Mutex<HashMap<String, Oid>>>,
}

#[cfg(feature = "postgres")]
/// Initialize the state required to use the sql!() macro.
pub fn init(connection: &Connection) {
    let query = "SELECT relname, oid
     FROM pg_class
     WHERE relkind = 'r'
        AND relowner = (
            SELECT usesysid
            FROM pg_user
            WHERE usename = CURRENT_USER
        )";
    let tables = table_singleton().inner;
    let mut tables = tables.lock().expect("table state");
    for row in &connection.query(query, &[]).unwrap() {
        tables.insert(row.get::<_, String>(0).to_lowercase(), row.get(1));
    }
}

#[cfg(feature = "postgres")]
fn table_singleton() -> TableState {
    // Initialize it to a null value
    static mut SINGLETON: *const TableState = 0 as *const TableState;
    static ONCE: Once = ONCE_INIT;

    unsafe {
        ONCE.call_once(|| {
            let singleton = TableState {
                inner: Arc::new(Mutex::new(HashMap::new())),
            };
            SINGLETON = mem::transmute(Box::new(singleton));
        });

        (*SINGLETON).clone()
    }
}
