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
pub type SqlFields = BTreeMap<String, Type>;

/// A collection of SQL tables.
pub type SqlTables = HashMap<String, SqlFields>;

/// A field type.
#[derive(Debug, Eq, PartialEq)]
pub enum Type {
    Bool,
    ByteString,
    Char,
    Custom(String),
    Dummy,
    F32,
    F64,
    I8,
    I16,
    I32,
    I64,
    Serial,
    String,
}

impl Display for Type {
    /// Get a string representation of the SQL `Type`.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let typ = match *self {
            Type::Bool => "bool",
            Type::ByteString => "Vec<u8>",
            Type::Char => "char",
            Type::Custom(ref typ) => &typ[..],
            Type::Dummy => "",
            Type::F32 => "f32",
            Type::F64 => "f64",
            Type::I8 => "i8",
            Type::I16 => "i16",
            Type::I32 => "i32",
            Type::I64 => "i64",
            Type::Serial => "i32",
            Type::String => "String",
        };
        write!(f, "{}", typ)
    }
}

/// Get the name of the primary key field.
pub fn get_primary_key_field(fields: &SqlFields) -> Option<String> {
    for (field, typ) in fields {
        if let Type::Serial = *typ {
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
