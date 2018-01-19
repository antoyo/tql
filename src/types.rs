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

#![allow(dead_code, non_camel_case_types)]

#[cfg(feature = "chrono")]
use chrono::{self, Local, NaiveDate, NaiveDateTime, NaiveTime, Utc};

type StdI8 = i8;

pub mod numbers {
    use super::ToTqlType;

    #[doc(hidden)]
    pub struct i8;

    impl ToTqlType for super::StdI8 {
        type Target = i8;
        fn to_tql_type(&self) -> Self::Target { i8 }
    }

    #[doc(hidden)]
    pub struct i16;

    impl ToTqlType for super::StdI16 {
        type Target = i16;
        fn to_tql_type(&self) -> Self::Target { i16 }
    }

    #[doc(hidden)]
    pub struct i32;

    impl ToTqlType for super::StdI32 {
        type Target = i32;
        fn to_tql_type(&self) -> Self::Target { i32 }
    }

    #[doc(hidden)]
    pub struct i64;

    impl ToTqlType for super::StdI64 {
        type Target = i64;
        fn to_tql_type(&self) -> Self::Target { i64 }
    }

    #[doc(hidden)]
    pub struct u8;

    impl ToTqlType for super::StdU8 {
        type Target = u8;
        fn to_tql_type(&self) -> Self::Target { u8 }
    }

    #[doc(hidden)]
    pub struct u16;

    impl ToTqlType for super::StdU16 {
        type Target = u16;
        fn to_tql_type(&self) -> Self::Target { u16 }
    }

    #[doc(hidden)]
    pub struct u32;

    impl ToTqlType for super::StdU32 {
        type Target = u32;
        fn to_tql_type(&self) -> Self::Target { u32 }
    }

    #[doc(hidden)]
    pub struct u64;

    impl ToTqlType for super::StdU64 {
        type Target = u64;
        fn to_tql_type(&self) -> Self::Target { u64 }
    }

    #[doc(hidden)]
    pub struct f32;

    impl ToTqlType for super::StdF32 {
        type Target = f32;
        fn to_tql_type(&self) -> Self::Target { f32 }
    }

    #[doc(hidden)]
    pub struct f64;

    impl ToTqlType for super::StdF64 {
        type Target = f64;
        fn to_tql_type(&self) -> Self::Target { f64 }
    }
}

type StdI16 = i16;

pub type StdI32 = i32;

type StdI64 = i64;

type StdU8 = u8;

type StdU16 = u16;

type StdU32 = u32;

type StdU64 = u64;

type StdF32 = f32;

type StdF64 = f64;

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

impl<T> ToTqlType for Option<T> {
    type Target = TqlOption;
    fn to_tql_type(&self) -> Self::Target { TqlOption }
}

#[doc(hidden)]
pub struct TqlString;

impl ToTqlType for String {
    type Target = TqlString;
    fn to_tql_type(&self) -> Self::Target { TqlString }
}

pub trait ToTqlType {
    type Target;

    fn to_tql_type(&self) -> Self::Target;
}
