//! A module providing SQL related functions.

// TODO: vérifier si les caractères spéciaux (\n, \t, …) doivent être échappés.

/// Escape the following characters: \ and '.
pub fn escape(string: String) -> String {
    string.replace("'", "''")
}
