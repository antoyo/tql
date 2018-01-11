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

//! These methods should not be used directly:
//! they exist only for type checking.

use types::{Date, DateTime, Time, TqlOption, TqlString};

impl Date {
    pub fn day(&self) -> i32 { 0 }
    pub fn month(&self) -> i32 { 0 }
    pub fn year(&self) -> i32 { 0 }
}

impl DateTime {
    pub fn day(&self) -> i32 { 0 }
    pub fn month(&self) -> i32 { 0 }
    pub fn year(&self) -> i32 { 0 }

    pub fn hour(&self) -> i32 { 0 }
    pub fn minute(&self) -> i32 { 0 }
    pub fn second(&self) -> i32 { 0 }
}

impl Time {
    pub fn hour(&self) -> i32 { 0 }
    pub fn minute(&self) -> i32 { 0 }
    pub fn second(&self) -> i32 { 0 }
}

impl TqlString {
    pub fn contains(&self, _string: &str) -> bool { false }
    pub fn ends_with(&self, _string: &str) -> bool { false }
    pub fn iregex(&self, _string: &str) -> bool { false }
    pub fn len(&self) -> usize { 0 }
    pub fn regex(&self, _string: &str) -> bool { false }
    pub fn starts_with(&self, _string: &str) -> bool { false }
}

impl TqlOption {
    pub fn is_some(&self) -> bool { false }
    pub fn is_none(&self) -> bool { false }
}
