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

use tql::{ForeignKey, PrimaryKey};

#[derive(SqlTable)]
pub struct TableModuleExpr {
    pub primary_key: PrimaryKey,
    pub field1: String,
    pub field2: i32,
    pub related_field: ForeignKey<RelatedTableModuleExpr>,
    pub optional_field: Option<i32>,
    pub boolean: Option<bool>,
    pub float64: Option<f64>,
    pub int16: Option<i16>,
    pub int64: Option<i64>,
}

#[derive(SqlTable)]
pub struct RelatedTableModuleExpr {
    pub primary_key: PrimaryKey,
    pub field1: i32,
}
