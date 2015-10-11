//! Global mutable state handling.
//!
//! The global state contains the SQL tables gathered by the `sql_table` attribute with their
//! fields.
//! A field is an identifier and a type.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::mem;

/// An SQL query argument.
pub struct SqlArg {
    pub high: u32,
    pub low: u32,
    pub name: String,
}

pub struct SqlArgs {
    pub arguments: Vec<Option<SqlArg>>,
    pub table_name: String,
}

/// A collection of fields.
pub type SqlFields = BTreeMap<String, Type>;

/// A collection of SQL tables.
pub type SqlTables = HashMap<String, SqlFields>;

/// A field type.
#[derive(Debug, Eq, PartialEq)]
pub enum Type {
    Dummy,
    I32,
    String,
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let typ = match *self {
            Type::Dummy => "",
            Type::I32 => "i32",
            Type::String => "String",
        };
        write!(f, "{}", typ)
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

/// Returns the global lint state.
pub fn lint_singleton() -> &'static mut SqlArgs {
    // FIXME: make this thread safe.
    static mut vector: *mut SqlArgs = 0 as *mut SqlArgs;

    let args = SqlArgs {
        arguments: vec![],
        table_name: "".to_string(),
    };
    unsafe {
        if vector == 0 as *mut SqlArgs {
            vector = mem::transmute(Box::new(args));
        }
        &mut *vector
    }
}
