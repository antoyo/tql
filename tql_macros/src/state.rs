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
//! The lint global state contains the arguments at every sql!() macro call site to be able to
//! analyze their types in the lint plugin.
//!
//! The methods global state contains the existing aggregate functions.
//!
//! The tables global state contains the SQL tables gathered by the `SqlTable` attribute with their
//! fields.
//! A field is an identifier and a type.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::mem;

use syn::{self, Ident, Span};

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

/// A tql method that can be used in filters.
pub type SqlMethod = HashMap<String, SqlMethodTypes>;

/// A collection mapping types to the methods that can be used on them.
pub type SqlMethods = HashMap<Type, SqlMethod>;

/// Tql method return type, argument types and template.
pub struct SqlMethodTypes {
    pub argument_types: Vec<Type>,
    pub return_type: Type,
    pub template: String,
}

/// An `SqlTable` has a name, a position and some `SqlFields`.
pub struct SqlTable {
    pub fields: SqlFields,
    pub name: Ident,
    pub position: Span,
}

/// A collection of SQL tables.
/// A map from table name to `SqlTable`.
pub type SqlTables = HashMap<String, SqlTable>;

/// Get the syn type of the field if it exists.
pub fn get_field_syn_type<'a, 'b>(table_name: &'a str, identifier: &'b Ident) -> Option<&'a syn::Type> {
    let tables = tables_singleton();
    tables.get(table_name)
        .and_then(|table| table.fields.get(identifier))
        .map(|ref types| &types.syn_type)
}

/// Get the type of the field if it exists.
pub fn get_field_type<'a, 'b>(table_name: &'a str, identifier: &'b Ident) -> Option<&'a Type> {
    let tables = tables_singleton();
    tables.get(table_name)
        .and_then(|table| table.fields.get(identifier))
        .map(|ref types| &types.ty.node)
}

/// Get method types by field name.
pub fn get_method_types<'a>(table_name: &str, field_name: &Ident, method_name: &str) -> Option<&'a SqlMethodTypes> {
    let tables = tables_singleton();
    let methods = methods_singleton();
    tables.get(table_name)
        .and_then(|table| table.fields.get(field_name))
        .and_then(move |ref types|
            methods.get(&types.ty.node)
                .and_then(|type_methods| type_methods.get(method_name))
        )
}

/// Get the name of the primary key field.
pub fn get_primary_key_field(table: &SqlTable) -> Option<&Ident> {
    table.fields.iter()
        .find(|&(_, ref types)| types.ty.node == Type::Serial)
        .map(|(field, _)| field)
}

/// Get the name of the primary key field by table name.
pub fn get_primary_key_field_by_table_name(table_name: &str) -> Option<String> {
    let tables = tables_singleton();
    tables.get(table_name).and_then(|table| get_primary_key_field(table)
                                    .map(|ident| ident.to_string()))
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

/// Returns the global state.
pub fn tables_singleton() -> &'static mut SqlTables {
    // FIXME: make this thread safe.
    static mut HASH_MAP: *mut SqlTables = 0 as *mut SqlTables;

    let map: SqlTables = HashMap::new();
    unsafe {
        if HASH_MAP == 0 as *mut SqlTables {
            HASH_MAP = mem::transmute(Box::new(map));
        }
        &mut *HASH_MAP
    }
}
