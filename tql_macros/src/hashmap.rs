/// An hashmap macro.

macro_rules! hashmap {
    { $($k:expr => $v:expr),* $(,)* } => {{
        let mut hashmap = ::std::collections::HashMap::new();
        $(hashmap.insert($k, $v);)*
        hashmap
    }};
}
