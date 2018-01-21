/*
 * Copyright (c) 2018 Boucher, Antoni <bouanto@zoho.com>
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

use std::fmt::Debug;
#[cfg(feature = "postgres")]
use std::io;

#[cfg(feature = "postgres")]
macro_rules! backend_extern_crate {
    () => { extern crate postgres; };
}

#[cfg(feature = "sqlite")]
macro_rules! backend_extern_crate {
    () => { extern crate rusqlite; };
}

#[cfg(feature = "postgres")]
use postgres::TlsMode;
#[cfg(feature = "postgres")]
pub use postgres::{Connection, Result};

#[cfg(feature = "postgres")]
#[allow(dead_code)]
pub fn get_connection() -> Connection {
    Connection::connect("postgres://test:test@localhost/database", TlsMode::None).unwrap()
}

#[cfg(feature = "sqlite")]
pub use rusqlite::{Connection, Error, Result};

#[cfg(feature = "sqlite")]
#[allow(dead_code)]
pub fn get_connection() -> Connection {
    Connection::open_in_memory().unwrap()
}

#[cfg(feature = "postgres")]
#[allow(dead_code)]
pub fn is_not_found<T: Debug>(result: Result<T>) -> bool {
    if let Err(error) = result {
        if let Some(io_error) = error.as_io() {
            return io_error.kind() == io::ErrorKind::NotFound;
        }
    }
    false
}

#[cfg(feature = "sqlite")]
#[allow(dead_code)]
pub fn is_not_found<T: Debug>(result: Result<T>) -> bool {
    if let Err(Error::QueryReturnedNoRows) = result {
        return true;
    }
    false
}
