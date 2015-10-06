use std::collections::BTreeMap;
use std::collections::HashMap;
use std::mem;

#[derive(Debug)]
pub enum Type {
    Dummy,
    Int,
    String,
}

pub type SqlFields = BTreeMap<String, Type>;
pub type SqlTables = HashMap<String, SqlFields>;

// FIXME: make this thread safe.
pub fn singleton() -> &'static mut SqlTables {
    static mut hash_map: *mut SqlTables = 0 as *mut SqlTables;

    let map: SqlTables = HashMap::new();
    unsafe {
        if hash_map == 0 as *mut SqlTables {
            hash_map = mem::transmute(Box::new(map));
        }
        &mut *hash_map
    }
}
