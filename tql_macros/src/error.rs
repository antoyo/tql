//! Error handling with the `Result` and `Error` types.
//!
//! `SqlResult<T>` is a `Result<T, Vec<Error>>` synonym and is used for returning and propagating
//! multiple compile errors.

use syntax::codemap::Span;

/// `Error` is a type that represents an error with its position.
pub struct Error<'a> {
    pub code: Option<&'a str>,
    pub kind: ErrorType,
    pub message: String,
    pub position: Span,
}

/// `ErrorType` is an `Error` type.
pub enum ErrorType {
    Error,
    Help,
    Note,
}

/// `SqlResult<T>` is a type that represents either a success (`Ok`) or failure (`Err`).
/// The failure may be represented by multiple `Error`s.
pub type SqlResult<'a, T> = Result<T, Vec<Error<'a>>>;

impl<'a> Error<'a> {
    /// Returns a new `Error`.
    ///
    /// This is a shortcut for:
    ///
    /// ```
    /// Error {
    ///     code: None,
    ///     message: message,
    ///     position: position,
    /// }
    /// ```
    pub fn new(message: String, position: Span) -> Error<'a> {
        Error {
            code: None,
            kind: ErrorType::Error,
            message: message,
            position: position,
        }
    }

    /// Returns a new `Error` of type help.
    ///
    /// This is a shortcut for:
    ///
    /// ```
    /// Error {
    ///     code: None,
    ///     message: message,
    ///     position: position,
    ///     type: Error::Note,
    /// }
    pub fn new_help(message: String, position: Span) -> Error<'a> {
        Error {
            code: None,
            kind: ErrorType::Help,
            message: message,
            position: position,
        }
    }

    /// Returns a new `Error` of type note.
    ///
    /// This is a shortcut for:
    ///
    /// ```
    /// Error {
    ///     code: None,
    ///     message: message,
    ///     position: position,
    ///     type: Error::Note,
    /// }
    pub fn new_note(message: String, position: Span) -> Error<'a> {
        Error {
            code: None,
            kind: ErrorType::Note,
            message: message,
            position: position,
        }
    }

    /// Returns a new `Error` with a code.
    ///
    /// This is a shortcut for:
    ///
    /// ```
    /// Error {
    ///     code: Some(code),
    ///     message: message,
    ///     position: position,
    /// }
    /// ```
    pub fn new_with_code(message: String, position: Span, code: &'a str) -> Error<'a> {
        Error {
            code: Some(code),
            kind: ErrorType::Error,
            message: message,
            position: position,
        }
    }
}

/// Returns an `SqlResult<T>` from potential result and errors.
/// Returns `Err` if there are at least one error.
/// Otherwise, returns `Ok`.
pub fn res<T>(result: T, errors: Vec<Error>) -> SqlResult<T> {
    if !errors.is_empty() {
        Err(errors)
    }
    else {
        Ok(result)
    }
}
