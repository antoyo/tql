use std::collections::HashSet;
use std::mem;

pub type SqlTables = HashSet<String>;

// FIXME: make this thread safe.
pub fn singleton() -> &'static mut SqlTables {
    static mut hash_map: *mut SqlTables = 0 as *mut SqlTables;

    let map: SqlTables = HashSet::new();
    unsafe {
        if hash_map == 0 as *mut SqlTables {
            hash_map = mem::transmute(Box::new(map));
        }
        &mut *hash_map
    }
}
