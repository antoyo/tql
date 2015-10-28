//! Methods definition.

use std::collections::HashMap;

use state::{SqlMethodTypes, methods_singleton};
use types::Type;

pub fn add_method(object_type: &Type, return_type: Type, argument_types: Vec<Type>, method: &str, template: &str) {
    let methods = methods_singleton();
    let type_methods = methods.entry(object_type.clone()).or_insert(HashMap::new());
    type_methods.insert(method.to_owned(), SqlMethodTypes {
        argument_types: argument_types,
        return_type: return_type,
        template: template.to_owned(),
    });
}

pub fn add_initial_methods() {
    let date_types = [Type::LocalDateTime, Type::NaiveDate, Type::NaiveDateTime, Type::UTCDateTime];
    for date_type in &date_types {
        // TODO: mettre ce code dans le module gen.
        add_method(date_type, Type::I32, vec![], "year", "EXTRACT(YEAR FROM $0)");
        add_method(date_type, Type::I32, vec![], "month", "EXTRACT(MONTH FROM $0)"); // TODO: utiliser le type U32.
        add_method(date_type, Type::I32, vec![], "day", "EXTRACT(DAY FROM $0)");
    }

    let time_types = [Type::LocalDateTime, Type::NaiveDateTime, Type::NaiveTime, Type::UTCDateTime];
    for time_type in &time_types {
        add_method(time_type, Type::I32, vec![], "hour", "EXTRACT(HOUR FROM $0)");
        add_method(time_type, Type::I32, vec![], "minute", "EXTRACT(MINUTE FROM $0)");
        add_method(time_type, Type::I32, vec![], "second", "EXTRACT(SECOND FROM $0)");
    }

    add_method(&Type::String, Type::Bool, vec![Type::String], "contains", "$0 LIKE '%' || $1 || '%'");
    add_method(&Type::String, Type::Bool, vec![Type::String], "ends_with", "$0 LIKE '%' || $1");
    add_method(&Type::String, Type::Bool, vec![Type::String], "starts_with", "$0 LIKE $1 || '%'");
    add_method(&Type::String, Type::I32, vec![], "len", "CHAR_LENGTH($0)");
    add_method(&Type::Nullable(box Type::Generic), Type::Bool, vec![], "is_some", "$0 IS NOT NULL");
    add_method(&Type::Nullable(box Type::Generic), Type::Bool, vec![], "is_none", "$0 IS NULL");
}
