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

//! SQL types.

use std::fmt::{self, Display, Formatter};

use rustc::middle::ty::{TypeAndMut, TyS, TypeVariants};
use syntax::ast::{AngleBracketedParameterData, FloatTy, IntTy, Path, PathParameters};
use syntax::ast::Expr_::ExprLit;
use syntax::ast::LitIntType::{SignedIntLit, UnsignedIntLit, UnsuffixedIntLit};
use syntax::ast::Lit_::{LitBool, LitByte, LitByteStr, LitChar, LitFloat, LitFloatUnsuffixed, LitInt, LitStr};
use syntax::ast::PathParameters::AngleBracketedParameters;
use syntax::ast::Ty_::TyPath;

use ast::Expression;
use gen::ToSql;
use state::{get_primary_key_field, tables_singleton};

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
    UTCDateTime,
}

impl Display for Type {
    /// Get a string representation of the SQL `Type` for display in error messages.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let typ = match *self {
            Type::Bool => "bool".to_owned(),
            Type::ByteString => "Vec<u8>".to_owned(),
            Type::Char => "char".to_owned(),
            Type::Custom(ref typ) => typ.clone(),
            Type::F32 => "f32".to_owned(),
            Type::F64 => "f64".to_owned(),
            Type::Generic => "".to_owned(),
            Type::I8 => "i8".to_owned(),
            Type::I16 => "i16".to_owned(),
            Type::I32 => "i32".to_owned(),
            Type::I64 => "i64".to_owned(),
            Type::LocalDateTime => "chrono::datetime::DateTime<chrono::offset::local::Local>".to_owned(),
            Type::NaiveDate => "chrono::naive::datetime::NaiveDate".to_owned(),
            Type::NaiveDateTime => "chrono::naive::datetime::NaiveDateTime".to_owned(),
            Type::NaiveTime => "chrono::naive::datetime::NaiveTime".to_owned(),
            Type::Nullable(ref typ) => "Option<".to_owned() + &typ.to_string() + ">",
            Type::Serial => "i32".to_owned(),
            Type::String => "String".to_owned(),
            Type::UnsupportedType(_) => "".to_owned(),
            Type::UTCDateTime => "chrono::datetime::DateTime<chrono::offset::utc::UTC>".to_owned(),
        };
        write!(f, "{}", typ)
    }
}

impl<'a> From<&'a Path> for Type {
    /// Convert a `Path` to a `Type`.
    fn from(&Path { ref segments, .. }: &Path) -> Type {
        let unsupported = Type::UnsupportedType("".to_owned());
        if segments.len() == 1 {
            let ident = segments[0].identifier.to_string();
            match &ident[..] {
                "bool" => Type::Bool,
                "char" => Type::Char,
                "DateTime" => match get_type_parameter(&segments[0].parameters) {
                    Some(ty) => match ty.as_ref() {
                        "Local" => Type::LocalDateTime,
                        "UTC" => Type::UTCDateTime,
                        parameter_type => Type::UnsupportedType("DateTime<".to_owned() + parameter_type + ">"),
                    },
                    None => Type::UnsupportedType("DateTime".to_owned()),
                },
                "f32" => Type::F32,
                "f64" => Type::F64,
                "i8" => Type::I8,
                "i16" => Type::I16,
                "i32" => Type::I32,
                "i64" => Type::I64,
                "ForeignKey" => match get_type_parameter(&segments[0].parameters) {
                    Some(ty) => Type::Custom(ty),
                    None => Type::UnsupportedType("ForeignKey".to_owned()),
                },
                "NaiveDate" => Type::NaiveDate,
                "NaiveDateTime" => Type::NaiveDateTime,
                "NaiveTime" => Type::NaiveTime,
                "Option" =>
                    match get_type_parameter_as_path(&segments[0].parameters) {
                        Some(ty) => {
                            let result = From::from(ty);
                            let typ =
                                if let Type::Nullable(_) = result {
                                    Type::UnsupportedType(result.to_string())
                                }
                                else {
                                    From::from(ty)
                                };
                            Type::Nullable(box typ)
                        },
                        None => Type::UnsupportedType("Option".to_owned()),
                    },
                "PrimaryKey" => {
                    Type::Serial
                },
                "String" => {
                    Type::String
                },
                "Vec" => match get_type_parameter(&segments[0].parameters) {
                    Some(ty) => match ty.as_ref() {
                        "u8" => Type::ByteString,
                        parameter_type => Type::UnsupportedType("Vec<".to_owned() + parameter_type + ">"),
                    },
                    None => Type::UnsupportedType("Vec".to_owned()),
                },
                typ => Type::UnsupportedType(typ.to_owned()), // TODO: show the generic types as well.
            }
        }
        else {
            unsupported
        }
    }
}

impl PartialEq<Expression> for Type {
    /// Check if an literal `expression` is equal to a `Type`.
    fn eq(&self, expression: &Expression) -> bool {
        // If the field type is `Nullable`, `expected_type` needs not to be an `Option`.
        let typ =
            match *self {
                Type::Nullable(box ref typ) => typ,
                ref typ => typ,
            };
        match expression.node {
            ExprLit(ref literal) => {
                match literal.node {
                    LitBool(_) => *typ == Type::Bool,
                    LitByte(_) => false,
                    LitByteStr(_) => *typ == Type::ByteString,
                    LitChar(_) => *typ == Type::Char,
                    LitFloat(_, FloatTy::TyF32) => *typ == Type::F32,
                    LitFloat(_, FloatTy::TyF64) => *typ == Type::F64,
                    LitFloatUnsuffixed(_) => *typ == Type::F32 || *typ == Type::F64,
                    LitInt(_, int_type) =>
                        match int_type {
                            SignedIntLit(IntTy::TyIs, _) => false,
                            SignedIntLit(IntTy::TyI8, _) => *typ == Type::I8,
                            SignedIntLit(IntTy::TyI16, _) => *typ == Type::I16,
                            SignedIntLit(IntTy::TyI32, _) => *typ == Type::I32 || *typ == Type::Serial,
                            SignedIntLit(IntTy::TyI64, _) => *typ == Type::I64,
                            UnsignedIntLit(_) => false,
                            UnsuffixedIntLit(_) =>
                                *typ == Type::I8 ||
                                *typ == Type::I16 ||
                                *typ == Type::I32 ||
                                *typ == Type::I64 ||
                                *typ == Type::Serial,
                        }
                    ,
                    LitStr(_, _) => *typ == Type::String,
                }
            }
            _ => true, // Returns true, because the type checking for non-literal is done later.
        }
    }
}

impl<'tcx> PartialEq<TyS<'tcx>> for Type {
    /// Compare the `expected_type` with `Type`.
    fn eq(&self, expected_type: &TyS<'tcx>) -> bool {
        // If the field type is `Nullable`, `expected_type` needs not to be an `Option`.
        let typ =
            match *self {
                Type::Nullable(box ref typ) => typ,
                ref typ => typ,
            };
        match expected_type.sty {
            TypeVariants::TyInt(IntTy::TyI32) => {
                match *typ {
                    Type::I32 | Type::Serial => true,
                    _ => false,
                }
            },
            TypeVariants::TyInt(IntTy::TyI64) => {
                *typ == Type::I64
            },
            TypeVariants::TyRef(_, TypeAndMut { ty, .. }) => {
                // TODO: support reference's reference.
                match ty.sty {
                    TypeVariants::TyStr => {
                        *typ == Type::String
                    },
                    _ => false,
                }
            },
            TypeVariants::TyStruct(def, sub) => {
                match def.struct_variant().name.to_string().as_str() {
                    "DateTime" => {
                        match sub.types.iter().next() {
                            Some(inner_type) => {
                                match get_type_parameter_from_type(&inner_type.sty) {
                                    Some(generic_type) =>
                                        match generic_type.as_str() {
                                            "UTC" => *typ == Type::UTCDateTime,
                                            "Local" => *typ == Type::LocalDateTime,
                                            _ => false,
                                        },
                                    _ => false,
                                }
                            },
                            None => false,
                        }
                    },
                    "NaiveDate" => *typ == Type::NaiveDate,
                    "NaiveDateTime" => *typ == Type::NaiveDateTime,
                    "NaiveTime" => *typ == Type::NaiveTime,
                    struct_type => *typ == Type::Custom(struct_type.to_owned()),
                }
            },
            _ => false,
        }
    }
}

/// Get the type between < and > as a String.
fn get_type_parameter(parameters: &PathParameters) -> Option<String> {
    get_type_parameter_as_path(parameters).map(|path| path.segments[0].identifier.to_string())
}

/// Get the type between < and > as a Path.
fn get_type_parameter_as_path(parameters: &PathParameters) -> Option<&Path> {
    if let AngleBracketedParameters(AngleBracketedParameterData { ref types, .. }) = *parameters {
        types.first()
            .and_then(|ty| {
                if let TyPath(None, ref path) = ty.node {
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

/// Get the type between < and > as a String.
fn get_type_parameter_from_type(typ: &TypeVariants) -> Option<String> {
    if let TypeVariants::TyStruct(def, _) = *typ {
        Some(def.struct_variant().name.to_string())
    }
    else {
        None
    }
}

/// Convert a `Type` to its SQL representation.
fn type_to_sql(typ: &Type, mut nullable: bool) -> String {
    let sql_type =
        match *typ {
            Type::Bool => "BOOLEAN".to_owned(),
            Type::ByteString => "BYTEA".to_owned(),
            Type::I8 | Type::Char => "CHARACTER(1)".to_owned(),
            Type::Custom(ref related_table_name) => {
                let tables = tables_singleton();
                if let Some(table) = tables.get(related_table_name) {
                    let primary_key_field = get_primary_key_field(table).unwrap();
                    "INTEGER REFERENCES ".to_owned() + &related_table_name + "(" + &primary_key_field + ")"
                }
                else {
                    "".to_owned()
                }
                // NOTE: if the field type is not an SQL table, an error is thrown by the linter.
            },
            Type::F32 => "REAL".to_owned(),
            Type::F64 => "DOUBLE PRECISION".to_owned(),
            Type::Generic => "".to_owned(),
            Type::I16 => "SMALLINT".to_owned(),
            Type::I32 => "INTEGER".to_owned(),
            Type::I64 => "BIGINT".to_owned(),
            Type::LocalDateTime => "TIMESTAMP WITH TIME ZONE".to_owned(),
            Type::NaiveDate => "DATE".to_owned(),
            Type::NaiveDateTime => "TIMESTAMP".to_owned(),
            Type::NaiveTime => "TIME".to_owned(),
            Type::Nullable(ref typ) => {
                nullable = true;
                type_to_sql(&*typ, true)
            },
            Type::Serial => "SERIAL PRIMARY KEY".to_owned(),
            Type::String => "CHARACTER VARYING".to_owned(),
            Type::UnsupportedType(_) => "".to_owned(), // TODO: should panic.
            Type::UTCDateTime => "TIMESTAMP WITH TIME ZONE".to_owned(),
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
