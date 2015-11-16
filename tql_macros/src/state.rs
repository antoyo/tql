/*
 * Copyright (C) 2015  Boucher, Antoni <bouanto@zoho.com>
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

//! Global mutable state handling.
//!
//! The global state contains the SQL tables gathered by the `sql_table` attribute with their
//! fields.
//! A field is an identifier and a type.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::mem;

use syntax::codemap::{Span, Spanned};

use methods::{add_initial_aggregates, add_initial_methods};
use types::Type;

/// A collection of tql aggregate functions.
pub type SqlAggregates = HashMap<String, String>;

/// An SQL query argument.
#[derive(Debug)]
pub struct SqlArg {
    pub high: u32,
    pub low: u32,
    pub typ: Type,
}

/// A collection of SQL query arguments.
#[derive(Debug)]
pub struct SqlArgs {
    pub arguments: Vec<SqlArg>,
    pub table_name: String,
}

/// A collection of query calls (with their arguments).
pub type SqlCalls = HashMap<u32, SqlArgs>;

/// A collection of fields.
pub type SqlFields = BTreeMap<String, Spanned<Type>>;

/// A tql method.
pub type SqlMethod = HashMap<String, SqlMethodTypes>;

/// A collection mapping tql methods to SQL functions.
pub type SqlMethods = HashMap<Type, SqlMethod>;

/// Tql method types.
pub struct SqlMethodTypes {
    pub argument_types: Vec<Type>,
    pub return_type: Type,
    pub template: String,
}

pub struct SqlTable {
    pub fields: SqlFields,
    pub name: String,
    pub position: Span,
}

/// A collection of SQL tables.
pub type SqlTables = HashMap<String, SqlTable>;

/// Get the type of the field if it exists.
pub fn get_field_type<'a, 'b>(table_name: &'a str, identifier: &'b str) -> Option<&'a Type> {
    let tables = singleton();
    tables.get(table_name)
        .and_then(|table| table.fields.get(identifier))
        .map(|field_type| &field_type.node)
}

/// Get method types by field name.
pub fn get_method_types<'a>(table_name: &str, field_name: &str, method_name: &str) -> Option<&'a SqlMethodTypes> {
    let tables = singleton();
    let methods = methods_singleton();
    tables.get(table_name)
        .and_then(|table| table.fields.get(field_name))
        .and_then(move |field_type|
            methods.get(&field_type.node)
                .and_then(|type_methods| type_methods.get(method_name))
        )
}

/// Get the name of the primary key field.
pub fn get_primary_key_field(table: &SqlTable) -> Option<String> {
    for (field, typ) in &table.fields {
        if let Type::Serial = typ.node {
            return Some(field.clone());
        }
    }
    None
}

/// Get the name of the primary key field by table name.
pub fn get_primary_key_field_by_table_name(table_name: &str) -> Option<String> {
    let tables = singleton();
    tables.get(table_name).and_then(|table| get_primary_key_field(table))
}

/// Returns the global aggregate state.
pub fn aggregates_singleton() -> &'static mut SqlAggregates {
    // FIXME: make this thread safe.
    static mut hash_map: *mut SqlAggregates = 0 as *mut SqlAggregates;

    let map: SqlAggregates = HashMap::new();
    unsafe {
        if hash_map == 0 as *mut SqlAggregates {
            hash_map = mem::transmute(Box::new(map));
            add_initial_aggregates();
        }
        &mut *hash_map
    }
}

/// Returns the global lint state.
pub fn lint_singleton() -> &'static mut SqlCalls {
    // FIXME: make this thread safe.
    static mut hash_map: *mut SqlCalls = 0 as *mut SqlCalls;

    let map: SqlCalls = HashMap::new();
    unsafe {
        if hash_map == 0 as *mut SqlCalls {
            hash_map = mem::transmute(Box::new(map));
        }
        &mut *hash_map
    }
}

/// Returns the global methods state.
pub fn methods_singleton() -> &'static mut SqlMethods {
    // FIXME: make this thread safe.
    static mut hash_map: *mut SqlMethods = 0 as *mut SqlMethods;

    let map: SqlMethods = HashMap::new();
    unsafe {
        if hash_map == 0 as *mut SqlMethods {
            hash_map = mem::transmute(Box::new(map));
            add_initial_methods();
        }
        &mut *hash_map
    }
}

/// Returns the global state.
pub fn singleton() -> &'static mut SqlTables {
    // FIXME: make this thread safe.
    static mut hash_map: *mut SqlTables = 0 as *mut SqlTables;

    let map: SqlTables = HashMap::new();
    unsafe {
        if hash_map == 0 as *mut SqlTables {
            hash_map = mem::transmute(Box::new(map));
        }
        &mut *hash_map
    }
}
