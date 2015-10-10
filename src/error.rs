//! Error handling with the `Result` and `Error` types.
//!
//! `SqlResult<T>` is a `Result<T, Vec<Error>>` synonym and is used for returning and propagating
//! multiple compile errors.

use syntax::codemap::Span;

/// `Error` is a type that represents an error with its position.
pub struct Error {
    pub message: String,
    pub position: Span,
}

/// `SqlResult<T>` is a type that represents either a success (`Ok`) or failure (`Err`).
/// The failure may be represented by multiple `Error`s.
pub type SqlResult<T> = Result<T, Vec<Error>>;

impl Error {
    /// Returns a new `Error`.
    ///
    /// This is a shortcut for:
    ///
    /// ```
    /// Error {
    ///     message: message,
    ///     position: position,
    /// }
    /// ```
    pub fn new(message: String, position: Span) -> Error {
        Error {
            message: message,
            position: position,
        }
    }
}

/// Returns an `SqlResult<T>` from potential result and errors.
/// Returns `Err` if there are at least one error.
/// Otherwise, returns `Ok`.
pub fn res<T>(result: T, errors: Vec<Error>) -> SqlResult<T> {
    if errors.len() > 0 {
        Err(errors)
    }
    else {
        Ok(result)
    }
}
