/*
 * Copyright (C) 2015  Boucher, Antoni <bouanto@zoho.com>
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

//! Error handling with the `Result` and `Error` types.
//!
//! `SqlResult<T>` is a `Result<T, Vec<Error>>` synonym and is used for returning and propagating
//! multiple compile errors.

use syntax::codemap::Span;

/// `Error` is a type that represents an error with its position.
#[derive(Debug)]
pub struct Error {
    pub code: Option<String>,
    pub kind: ErrorType, // TODO: use an enum.
    pub message: String,
    pub position: Span,
}

/// `ErrorType` is an `Error` type.
#[derive(Debug)]
pub enum ErrorType {
    Error,
    Help,
    Note,
    Warning,
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
    ///     code: None,
    ///     kind: ErrorType::Error,
    ///     message: message,
    ///     position: position,
    /// }
    /// ```
    pub fn new(message: String, position: Span) -> Error {
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
    ///     kind: ErrorType::Note,
    ///     message: message,
    ///     position: position,
    /// }
    pub fn new_help(message: String, position: Span) -> Error {
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
    ///     kind: ErrorType::Note,
    ///     message: message,
    ///     position: position,
    /// }
    pub fn new_note(message: String, position: Span) -> Error {
        Error {
            code: None,
            kind: ErrorType::Note,
            message: message,
            position: position,
        }
    }

    /// Returns a new `Error` of type warning.
    ///
    /// This is a shortcut for:
    ///
    /// ```
    /// Error {
    ///     code: None,
    ///     kind: ErrorType::Warning,
    ///     message: message,
    ///     position: position,
    /// }
    pub fn new_warning(message: String, position: Span) -> Error {
        Error {
            code: None,
            kind: ErrorType::Warning,
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
    ///     code: Some(code.to_owned()),
    ///     kind: ErrorType::Error,
    ///     message: message,
    ///     position: position,
    /// }
    /// ```
    pub fn new_with_code(message: String, position: Span, code: &str) -> Error {
        Error {
            code: Some(code.to_owned()),
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
