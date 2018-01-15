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

use std::fmt::{self, Display, Formatter};

use syn::{
    self,
    AngleBracketedGenericArguments,
    Expr,
    ExprLit,
    FloatSuffix,
    GenericArgument,
    IntSuffix,
    Lit,
    Path,
    PathArguments,
    TypePath,
};

use ast::Expression;
use sql::ToSql;

/// A field type.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Type {
    Bool,
    ByteString,
    Char,
    Custom(String),
    F32,
    F64,
    Generic,
    I8,
    I16,
    I32,
    I64,
    LocalDateTime,
    NaiveDate,
    NaiveDateTime,
    NaiveTime,
    Nullable(Box<Type>),
    Serial,
    String,
    UnsupportedType(String),
    UtcDateTime,
}

impl Display for Type {
    /// Get a string representation of the SQL `Type` for display in error messages.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let typ = match *self {
            Type::Bool => "bool".to_string(),
            Type::ByteString => "Vec<u8>".to_string(),
            Type::Char => "char".to_string(),
            Type::Custom(ref typ) => typ.clone(),
            Type::F32 => "f32".to_string(),
            Type::F64 => "f64".to_string(),
            Type::Generic => "".to_string(),
            Type::I8 => "i8".to_string(),
            Type::I16 => "i16".to_string(),
            Type::I32 => "i32".to_string(),
            Type::I64 => "i64".to_string(),
            Type::LocalDateTime => "chrono::datetime::DateTime<chrono::offset::Local>".to_string(),
            Type::NaiveDate => "chrono::naive::NaiveDate".to_string(),
            Type::NaiveDateTime => "chrono::naive::NaiveDateTime".to_string(),
            Type::NaiveTime => "chrono::naive::NaiveTime".to_string(),
            Type::Nullable(ref typ) => "Option<".to_string() + &typ.to_string() + ">",
            Type::Serial => "i32".to_string(),
            Type::String => "String".to_string(),
            Type::UnsupportedType(_) => "".to_string(),
            Type::UtcDateTime => "chrono::datetime::DateTime<chrono::offset::Utc>".to_string(),
        };
        write!(f, "{}", typ)
    }
}

/// Convert a `Type` to its SQL representation.
fn type_to_sql(typ: &Type, mut nullable: bool) -> String {
    let sql_type =
        match *typ {
            Type::Bool => "BOOLEAN".to_string(),
            Type::ByteString => "BYTEA".to_string(),
            Type::I8 | Type::Char => "CHARACTER(1)".to_string(),
            Type::Custom(ref related_table_name) => {
                "INTEGER REFERENCES ".to_string() + related_table_name + "({" + related_table_name + "_pk})"
                // NOTE: if the field type is not an SQL table, an error is thrown by the linter.
                // FIXME: previous comment.
            },
            Type::F32 => "REAL".to_string(),
            Type::F64 => "DOUBLE PRECISION".to_string(),
            Type::Generic => "".to_string(),
            Type::I16 => "SMALLINT".to_string(),
            Type::I32 => "INTEGER".to_string(),
            Type::I64 => "BIGINT".to_string(),
            Type::LocalDateTime => "TIMESTAMP WITH TIME ZONE".to_string(),
            Type::NaiveDate => "DATE".to_string(),
            Type::NaiveDateTime => "TIMESTAMP".to_string(),
            Type::NaiveTime => "TIME".to_string(),
            Type::Nullable(ref typ) => {
                nullable = true;
                type_to_sql(&*typ, true)
            },
            Type::Serial => "SERIAL PRIMARY KEY".to_string(),
            Type::String => "CHARACTER VARYING".to_string(),
            Type::UnsupportedType(_) => "".to_string(), // TODO: should panic.
            Type::UtcDateTime => "TIMESTAMP WITH TIME ZONE".to_string(),
        };

    if nullable {
        sql_type
    }
    else {
        sql_type + " NOT NULL"
    }
}

impl ToSql for Type {
    fn to_sql(&self) -> String {
        type_to_sql(self, false)
    }
}

impl PartialEq<Expression> for Type {
    /// Check if an literal `expression` is equal to a `Type`.
    fn eq(&self, expression: &Expression) -> bool {
        // If the field type is `Nullable`, `expected_type` needs not to be an `Option`.
        let typ =
            match *self {
                Type::Nullable(ref typ) => typ,
                ref typ => typ,
            };
        match *expression {
            Expr::Lit(ExprLit { lit: Lit::Bool(_), .. }) => *typ == Type::Bool,
            Expr::Lit(ExprLit { lit: Lit::Byte(_), .. }) => false,
            Expr::Lit(ExprLit { lit: Lit::ByteStr(_), .. }) => *typ == Type::ByteString,
            Expr::Lit(ExprLit { lit: Lit::Char(_), .. }) => *typ == Type::Char,
            Expr::Lit(ExprLit { lit: Lit::Float(ref float), .. }) =>
                match float.suffix() {
                    // TODO: check if right suffix.
                    FloatSuffix::F32 => *typ == Type::F32,
                    FloatSuffix::F64 => *typ == Type::F64,
                    FloatSuffix::None => *typ == Type::F32 || *typ == Type::F64,
                },
            Expr::Lit(ExprLit { lit: Lit::Int(ref int), .. }) =>
                match int.suffix() {
                    IntSuffix::Isize => false,
                    IntSuffix::I8 => *typ == Type::I8,
                    IntSuffix::I16 => *typ == Type::I16,
                    IntSuffix::I32 => *typ == Type::I32 || *typ == Type::Serial,
                    IntSuffix::I64 => *typ == Type::I64,
                    IntSuffix::U8 | IntSuffix::U16 | IntSuffix::U32 | IntSuffix::U64 | IntSuffix::U128 |
                        IntSuffix::Usize | IntSuffix::I128 => false,
                    IntSuffix::None =>
                        *typ == Type::I8 ||
                        *typ == Type::I16 ||
                        *typ == Type::I32 ||
                        *typ == Type::I64 ||
                        *typ == Type::Serial,
                },
            Expr::Lit(ExprLit { lit: Lit::Str(_), .. }) => *typ == Type::String,
            _ => true, // Returns true, because the type checking for non-literal is done later.
        }
    }
}

impl<'a> From<&'a Path> for Type {
    /// Convert a `Path` to a `Type`.
    fn from(&Path { ref segments, .. }: &Path) -> Type {
        let unsupported = Type::UnsupportedType("".to_string());
        if segments.len() == 1 {
            let element = segments.first().expect("first segment of path");
            let first_segment = element.value();
            let ident = first_segment.ident.to_string();
            match &ident[..] {
                "bool" => Type::Bool,
                "char" => Type::Char,
                "DateTime" => match get_type_parameter(&first_segment.arguments) {
                    Some(ty) => match ty.as_ref() {
                        "Local" => Type::LocalDateTime,
                        "Utc" => Type::UtcDateTime,
                        parameter_type => Type::UnsupportedType("DateTime<".to_string() + parameter_type + ">"),
                    },
                    None => Type::UnsupportedType("DateTime".to_string()),
                },
                "f32" => Type::F32,
                "f64" => Type::F64,
                "i8" => Type::I8,
                "i16" => Type::I16,
                "i32" => Type::I32,
                "i64" => Type::I64,
                "ForeignKey" => match get_type_parameter(&first_segment.arguments) {
                    Some(ty) => Type::Custom(ty),
                    None => Type::UnsupportedType("ForeignKey".to_string()),
                },
                "NaiveDate" => Type::NaiveDate,
                "NaiveDateTime" => Type::NaiveDateTime,
                "NaiveTime" => Type::NaiveTime,
                "Option" =>
                    match get_type_parameter_as_path(&first_segment.arguments) {
                        Some(ty) => {
                            let result = From::from(ty);
                            let typ =
                                if let Type::Nullable(_) = result {
                                    Type::UnsupportedType(result.to_string())
                                }
                                else {
                                    From::from(ty)
                                };
                            Type::Nullable(Box::new(typ))
                        },
                        None => Type::UnsupportedType("Option".to_string()),
                    },
                "PrimaryKey" => {
                    Type::Serial
                },
                "String" => {
                    Type::String
                },
                "Vec" => match get_type_parameter(&first_segment.arguments) {
                    Some(ty) => match ty.as_ref() {
                        "u8" => Type::ByteString,
                        parameter_type => Type::UnsupportedType("Vec<".to_string() + parameter_type + ">"),
                    },
                    None => Type::UnsupportedType("Vec".to_string()),
                },
                typ => Type::UnsupportedType(typ.to_string()), // TODO: show the generic types as well.
            }
        }
        else {
            unsupported
        }
    }
}

/// Get the type between < and > as a String.
pub fn get_type_parameter(parameters: &PathArguments) -> Option<String> {
    get_type_parameter_as_path(parameters).map(|path| path.segments.first()
        .expect("first segment in path").value().ident.to_string())
}

/// Get the type between < and > as a Path.
pub fn get_type_parameter_as_path(parameters: &PathArguments) -> Option<&Path> {
    if let PathArguments::AngleBracketed(AngleBracketedGenericArguments { ref args, .. }) = *parameters {
        args.first()
            .and_then(|ty| {
                if let GenericArgument::Type(syn::Type::Path(TypePath { ref path, .. })) = **ty.value() {
                    Some(path)
                }
                else {
                    None
                }
            })
    }
    else {
        None
    }
}
