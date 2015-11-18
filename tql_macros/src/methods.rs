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

//! Methods definition for use in filters.

use std::collections::HashMap;

use state::{SqlMethodTypes, aggregates_singleton, methods_singleton};
use types::Type;

/// Add a new `method` on `object_type` of type `argument_types` -> `return_type`.
/// The template is the resulting SQL with `$0` as a placeholder for `self` and `$1`, `$2`, … as
/// placeholders for the arguments.
pub fn add_method(object_type: &Type, return_type: Type, argument_types: Vec<Type>, method: &str, template: &str) {
    let methods = methods_singleton();
    let type_methods = methods.entry(object_type.clone()).or_insert(HashMap::new());
    type_methods.insert(method.to_owned(), SqlMethodTypes {
        argument_types: argument_types,
        return_type: return_type,
        template: template.to_owned(),
    });
}

/// Add a new aggregate `rust_function` mapping to `sql_function`.
pub fn add_aggregate(rust_function: &str, sql_function: &str) {
    let aggregates = aggregates_singleton();
    aggregates.insert(rust_function.to_owned(), sql_function.to_owned());
}

/// Add the default SQL aggregate functions.
pub fn add_initial_aggregates() {
    add_aggregate("avg", "AVG");
}

/// Add the default SQL methods.
pub fn add_initial_methods() {
    // Date methods.
    let date_types = [Type::LocalDateTime, Type::NaiveDate, Type::NaiveDateTime, Type::UTCDateTime];
    for date_type in &date_types {
        // TODO: put this code in the gen module.
        add_method(date_type, Type::I32, vec![], "year", "EXTRACT(YEAR FROM $0)");
        add_method(date_type, Type::I32, vec![], "month", "EXTRACT(MONTH FROM $0)"); // TODO: use the U32 type.
        add_method(date_type, Type::I32, vec![], "day", "EXTRACT(DAY FROM $0)");
    }

    // Time methods.
    let time_types = [Type::LocalDateTime, Type::NaiveDateTime, Type::NaiveTime, Type::UTCDateTime];
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
    add_method(&Type::String, Type::Bool, vec![Type::String], "match", "$0 LIKE $1");
    add_method(&Type::String, Type::Bool, vec![Type::String], "imatch", "$0 ILIKE $1");

    // Option methods.
    add_method(&Type::Nullable(box Type::Generic), Type::Bool, vec![], "is_some", "$0 IS NOT NULL");
    add_method(&Type::Nullable(box Type::Generic), Type::Bool, vec![], "is_none", "$0 IS NULL");
}
