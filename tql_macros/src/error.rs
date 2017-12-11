/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Error handling with the `Result` and `Error` types.
//!
//! `Result<T>` is a `Result<T, Vec<Error>>` synonym and is used for returning and propagating
//! multiple compile errors.

use std::result;

#[cfg(feature = "unstable")]
use proc_macro::{Diagnostic, Level};
#[cfg(not(feature = "unstable"))]
use quote::Tokens;
use syn::Span;

#[cfg(feature = "unstable")]
use to_proc_macro_span;

/// `Error` is a type that represents an error with its position.
#[derive(Debug)]
pub struct Error {
    pub code: Option<String>,
    pub children: Vec<Error>,
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

/// `Result<T>` is a type that represents either a success (`Ok`) or failure (`Err`).
/// The failure may be represented by multiple `Error`s.
pub type Result<T> = result::Result<T, Vec<Error>>;

impl Error {
    /// Returns a new `Error`.
    ///
    /// This is a shortcut for:
    ///
    /// ```
    /// Error {
    ///     code: None,
    ///     children: vec![],
    ///     kind: ErrorType::Error,
    ///     message: message,
    ///     position,
    /// }
    /// ```
    pub fn new(message: &str, position: Span) -> Self {
        Self {
            code: None,
            children: vec![],
            kind: ErrorType::Error,
            message: message.to_string(),
            position,
        }
    }

    /// Add a help children message to the current error.
    pub fn add_help(&mut self, message: &str) {
        self.children.push(Self {
            code: None,
            children: vec![],
            kind: ErrorType::Help,
            message: message.to_string(),
            position: Span::default(),
        });
    }

    /// Add a note children message to the current error.
    pub fn add_note(&mut self, message: &str) {
        self.children.push(Self {
            code: None,
            children: vec![],
            kind: ErrorType::Note,
            message: message.to_string(),
            position: Span::default(),
        });
    }

    /// Returns a new `Error` of type warning.
    ///
    /// This is a shortcut for:
    ///
    /// ```
    /// Error {
    ///     code: None,
    ///     children: vec![],
    ///     kind: ErrorType::Warning,
    ///     message: message,
    ///     position,
    /// }
    pub fn new_warning(message: &str, position: Span) -> Self {
        Self {
            code: None,
            children: vec![],
            kind: ErrorType::Warning,
            message: message.to_string(),
            position,
        }
    }

    /// Returns a new `Error` with a code.
    ///
    /// This is a shortcut for:
    ///
    /// ```
    /// Error {
    ///     code: Some(code.to_string()),
    ///     children: vec![],
    ///     kind: ErrorType::Error,
    ///     message: message,
    ///     position,
    /// }
    /// ```
    pub fn new_with_code(message: &str, position: Span, code: &str) -> Self {
        Self {
            code: Some(code.to_string()),
            children: vec![],
            kind: ErrorType::Error,
            message: message.to_string(),
            position,
        }
    }

    #[cfg(feature = "unstable")]
    pub fn emit_diagnostic(self) {
        let span = to_proc_macro_span(self.position);
        let mut diagnostic = Diagnostic::spanned(span, self.kind.into(), self.message);
        for child in self.children {
            let func =
                match child.kind {
                    ErrorType::Error => Diagnostic::error,
                    ErrorType::Help => Diagnostic::help,
                    ErrorType::Note => Diagnostic::note,
                    ErrorType::Warning => Diagnostic::warning,
                };
            diagnostic = func(diagnostic, child.message);
        }
        diagnostic.emit();
    }

    #[cfg(not(feature = "unstable"))]
    fn to_string(&self) -> String {
        let kind =
            match self.kind {
                ErrorType::Error => "",
                ErrorType::Help => "help: ",
                ErrorType::Note => "note: ",
                ErrorType::Warning => "warning: ",
            };
        let mut messages = format!("{}{}", kind, self.message);
        for child in &self.children {
            messages += &format!("\n{}", child.to_string());
        }
        messages
    }
}

/// Returns an `Result<T>` from potential result and errors.
/// Returns `Err` if there are at least one error.
/// Otherwise, returns `Ok`.
pub fn res<T>(result: T, errors: Vec<Error>) -> Result<T> {
    if !errors.is_empty() {
        Err(errors)
    }
    else {
        Ok(result)
    }
}

#[cfg(not(feature = "unstable"))]
pub fn compiler_error(msg: &Error) -> Tokens {
    let msg = msg.to_string();
    quote! {
        compile_error!(#msg);
    }
}

#[cfg(feature = "unstable")]
impl From<ErrorType> for Level {
    fn from(error_type: ErrorType) -> Self {
        match error_type {
            ErrorType::Error => Level::Error,
            ErrorType::Help => Level::Help,
            ErrorType::Note => Level::Note,
            ErrorType::Warning => Level::Warning,
        }
    }
}
