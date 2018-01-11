/*
 * Copyright (c) 2018 Boucher, Antoni <bouanto@zoho.com>
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

//! These types should not be used directly:
//! they exist only for type checking.

#[cfg(feature = "chrono")]
use chrono::{self, Local, NaiveDate, NaiveDateTime, NaiveTime, Utc};

#[doc(hidden)]
pub struct Int;

impl ToTqlType for i8 {
    type Target = Int;
    fn to_tql_type(&self) -> Self::Target { Int }
}

impl ToTqlType for i16 {
    type Target = Int;
    fn to_tql_type(&self) -> Self::Target { Int }
}

impl ToTqlType for i32 {
    type Target = Int;
    fn to_tql_type(&self) -> Self::Target { Int }
}

impl ToTqlType for i64 {
    type Target = Int;
    fn to_tql_type(&self) -> Self::Target { Int }
}

impl ToTqlType for u8 {
    type Target = Int;
    fn to_tql_type(&self) -> Self::Target { Int }
}

impl ToTqlType for u16 {
    type Target = Int;
    fn to_tql_type(&self) -> Self::Target { Int }
}

impl ToTqlType for u32 {
    type Target = Int;
    fn to_tql_type(&self) -> Self::Target { Int }
}

impl ToTqlType for u64 {
    type Target = Int;
    fn to_tql_type(&self) -> Self::Target { Int }
}

#[doc(hidden)]
pub struct Float;

impl ToTqlType for f32 {
    type Target = Float;
    fn to_tql_type(&self) -> Self::Target { Float }
}

impl ToTqlType for f64 {
    type Target = Float;
    fn to_tql_type(&self) -> Self::Target { Float }
}

#[doc(hidden)]
pub struct Date;

#[cfg(feature = "chrono")]
impl ToTqlType for chrono::Date<Local> {
    type Target = Date;
    fn to_tql_type(&self) -> Self::Target { Date }
}

#[cfg(feature = "chrono")]
impl ToTqlType for chrono::Date<Utc> {
    type Target = Date;
    fn to_tql_type(&self) -> Self::Target { Date }
}

#[cfg(feature = "chrono")]
impl ToTqlType for NaiveDate {
    type Target = Date;
    fn to_tql_type(&self) -> Self::Target { Date }
}

#[doc(hidden)]
pub struct DateTime;

#[cfg(feature = "chrono")]
impl ToTqlType for chrono::DateTime<Local> {
    type Target = DateTime;
    fn to_tql_type(&self) -> Self::Target { DateTime }
}

#[cfg(feature = "chrono")]
impl ToTqlType for chrono::DateTime<Utc> {
    type Target = DateTime;
    fn to_tql_type(&self) -> Self::Target { DateTime }
}

#[cfg(feature = "chrono")]
impl ToTqlType for NaiveDateTime {
    type Target = DateTime;
    fn to_tql_type(&self) -> Self::Target { DateTime }
}

#[doc(hidden)]
pub struct Time;

#[cfg(feature = "chrono")]
impl ToTqlType for NaiveTime {
    type Target = Time;
    fn to_tql_type(&self) -> Self::Target { Time }
}

#[doc(hidden)]
pub struct TqlOption;

#[cfg(feature = "chrono")]
impl<T> ToTqlType for Option<T> {
    type Target = TqlOption;
    fn to_tql_type(&self) -> Self::Target { TqlOption }
}

#[doc(hidden)]
pub struct TqlString;

#[cfg(feature = "chrono")]
impl ToTqlType for String {
    type Target = TqlString;
    fn to_tql_type(&self) -> Self::Target { TqlString }
}

pub trait ToTqlType {
    type Target;

    fn to_tql_type(&self) -> Self::Target;
}
