//! A module providing SQL related functions.

// TODO: check if special characters (\n, \t, …) should be escaped.

/// Escape the following characters: \ and '.
pub fn escape(string: String) -> String {
    string.replace("'", "''")
}
