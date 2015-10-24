//! TQL is a Rust compiler plugin providing an SQL DSL.
//!
//! It type check your expression at compile time and converts it to SQL.

/// The `ForeignKey` is optional.
///
/// There is no value when the `join()` method is not called.
pub type ForeignKey<T> = Option<T>;

/// A `PrimaryKey` is a 4-byte integer.
pub type PrimaryKey = i32;
