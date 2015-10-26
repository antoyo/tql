//! Global mutable state handling.
//!
//! The global state contains the SQL tables gathered by the `sql_table` attribute with their
//! fields.
//! A field is an identifier and a type.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::mem;

use syntax::codemap::Spanned;

use methods::add_initial_methods;
use types::Type;

/// An SQL query argument.
#[derive(Debug)]
pub struct SqlArg {
    pub high: u32,
    pub low: u32,
    pub name: String,
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

/// A collection mapping tql methods to SQL functions.
pub type SqlMethods = HashMap<Type, HashMap<String, String>>;

/// A collection of SQL tables.
pub type SqlTables = HashMap<String, SqlFields>;

/// Get the name of the primary key field.
pub fn get_primary_key_field(fields: &SqlFields) -> Option<String> {
    for (field, typ) in fields {
        if let Type::Serial = typ.node {
            return Some(field.clone());
        }
    }
    None
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

/// Returns the global lint state.
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
