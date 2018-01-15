/*
 * Copyright (c) 2017-2018 Boucher, Antoni <bouanto@zoho.com>
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
    const FIELD_COUNT: usize;

    fn _create_query() -> String;

    fn default() -> Self;

    #[cfg(feature = "postgres")]
    fn from_row(row: &::postgres::rows::Row) -> Self;

    #[cfg(feature = "postgres")]
    fn from_related_row(row: &::postgres::rows::Row, delta: usize) -> Self;

    fn field_list() -> &'static str;
}

#[cfg(feature = "postgres")]
#[doc(hidden)]
pub fn from_related_row<T: SqlTable>(field: &mut Option<T>, row: &::postgres::rows::Row, delta: usize) -> usize
{
    *field = Some(T::from_related_row(row, delta));
    T::FIELD_COUNT
}

// Stable implementation.

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
