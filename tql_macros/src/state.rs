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

//! Global mutable states handling.
//!
//! There are four global states:
//!
//! The aggregates global state contains the existing aggregate functions.
//!
//! The methods global state contains the existing aggregate functions.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::mem;

use syn::{self, Ident};

use ast::WithSpan;
use methods::{add_initial_aggregates, add_initial_methods};
use types::Type;

/// A collection of tql aggregate functions.
pub type SqlAggregates = HashMap<String, String>;

#[derive(Debug)]
pub struct BothTypes {
    pub syn_type: syn::Type,
    pub ty: WithSpan<Type>,
}

/// A collection of fields from an `SqlTable`.
pub type SqlFields = BTreeMap<Ident, BothTypes>;

/// A collection mapping method names to methods.
pub type SqlMethods = HashMap<String, SqlMethodTypes>;

/// Tql method return type, argument types and template.
pub struct SqlMethodTypes {
    pub argument_types: Vec<Type>,
    pub object_type: Type,
    pub return_type: Type,
    pub template: Option<String>,
}

/// Returns the global aggregate state.
pub fn aggregates_singleton() -> &'static mut SqlAggregates {
    // FIXME: make this thread safe.
    static mut HASH_MAP: *mut SqlAggregates = 0 as *mut SqlAggregates;

    let map: SqlAggregates = HashMap::new();
    unsafe {
        if HASH_MAP == 0 as *mut SqlAggregates {
            HASH_MAP = mem::transmute(Box::new(map));
            add_initial_aggregates();
        }
        &mut *HASH_MAP
    }
}

/// Returns the global methods state.
pub fn methods_singleton() -> &'static mut SqlMethods {
    // FIXME: make this thread safe.
    static mut HASH_MAP: *mut SqlMethods = 0 as *mut SqlMethods;

    let map: SqlMethods = HashMap::new();
    unsafe {
        if HASH_MAP == 0 as *mut SqlMethods {
            HASH_MAP = mem::transmute(Box::new(map));
            add_initial_methods();
        }
        &mut *HASH_MAP
    }
}
