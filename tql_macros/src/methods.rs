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

//! Methods definition for use in filters.

use std::collections::HashMap;

use state::{SqlMethodTypes, aggregates_singleton, methods_singleton};
use types::Type;

/// Add a new `method` on `object_type` of type `argument_types` -> `return_type`.
/// The template is the resulting SQL with `$0` as a placeholder for `self` and `$1`, `$2`, … as
/// placeholders for the arguments.
pub fn add_method(object_type: &Type, return_type: Type, argument_types: Vec<Type>, method: &str, template: &str) {
    let methods = methods_singleton();
    methods.insert(method.to_string(), SqlMethodTypes {
        argument_types,
        object_type: object_type.clone(),
        return_type,
        template: template.to_string(),
    });
}

/// Add a new aggregate `rust_function` mapping to `sql_function`.
pub fn add_aggregate(rust_function: &str, sql_function: &str) {
    let aggregates = aggregates_singleton();
    aggregates.insert(rust_function.to_string(), sql_function.to_string());
}

/// Add the default SQL aggregate functions.
pub fn add_initial_aggregates() {
    add_aggregate("avg", "AVG");
}

/// Add the default SQL methods.
pub fn add_initial_methods() {
    // Date methods.
    let date_types = [Type::LocalDateTime, Type::NaiveDate, Type::NaiveDateTime, Type::UtcDateTime];
    for date_type in &date_types {
        // TODO: put this code in the gen module.
        add_method(date_type, Type::I32, vec![], "year", "EXTRACT(YEAR FROM $0)");
        add_method(date_type, Type::I32, vec![], "month", "EXTRACT(MONTH FROM $0)"); // TODO: use the U32 type.
        add_method(date_type, Type::I32, vec![], "day", "EXTRACT(DAY FROM $0)");
    }

    // Time methods.
    let time_types = [Type::LocalDateTime, Type::NaiveDateTime, Type::NaiveTime, Type::UtcDateTime];
    for time_type in &time_types {
        add_method(time_type, Type::I32, vec![], "hour", "EXTRACT(HOUR FROM $0)");
        add_method(time_type, Type::I32, vec![], "minute", "EXTRACT(MINUTE FROM $0)");
        add_method(time_type, Type::I32, vec![], "second", "EXTRACT(SECOND FROM $0)");
    }

    // TODO: perhaps create a macro to ease the method creation.
    // For instance :
    // add_method!(impl Type::String {
    //     fn contains(Type::String) -> Type::Bool {
    //         "$0 LIKE '%' || $1 || '%'"
    //     }
    // });

    // String methods.
    add_method(&Type::String, Type::Bool, vec![Type::String], "contains", "$0 LIKE '%' || $1 || '%'");
    add_method(&Type::String, Type::Bool, vec![Type::String], "ends_with", "$0 LIKE '%' || $1");
    add_method(&Type::String, Type::Bool, vec![Type::String], "starts_with", "$0 LIKE $1 || '%'");
    add_method(&Type::String, Type::I32, vec![], "len", "CHAR_LENGTH($0)");
    add_method(&Type::String, Type::Bool, vec![Type::String], "regex", "$0 LIKE $1");
    add_method(&Type::String, Type::Bool, vec![Type::String], "iregex", "$0 ILIKE $1");

    // Option methods.
    add_method(&Type::Nullable(Box::new(Type::Generic)), Type::Bool, vec![], "is_some", "$0 IS NOT NULL");
    add_method(&Type::Nullable(Box::new(Type::Generic)), Type::Bool, vec![], "is_none", "$0 IS NULL");
}
