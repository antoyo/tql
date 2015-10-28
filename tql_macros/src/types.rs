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
use state::{get_primary_key_field, singleton};

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
            Type::Nullable(ref typ) => typ.to_string(),
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
                "DateTime" => match get_type_parameter(&segments[0].parameters) {
                    Some(ty) => match ty.as_ref() {
                        "Local" => Type::LocalDateTime,
                        "UTC" => Type::UTCDateTime,
                        _ => unsupported, // TODO
                    },
                    None => unsupported, // TODO
                },
                "i32" => {
                    Type::I32
                },
                "ForeignKey" => match get_type_parameter(&segments[0].parameters) {
                    Some(ty) => Type::Custom(ty),
                    None => unsupported, // TODO
                },
                "NaiveDate" => Type::NaiveDate,
                "NaiveDateTime" => Type::NaiveDateTime,
                "NaiveTime" => Type::NaiveTime,
                "Option" => match get_type_parameter_as_path(&segments[0].parameters) {
                    Some(ty) => {
                        let result = From::from(ty);
                        if let Type::UnsupportedType(_) = result {
                            result
                        }
                        else {
                            Type::Nullable(box result)
                        }
                    },
                    None => unsupported, // TODO
                },
                "PrimaryKey" => {
                    Type::Serial
                },
                "String" => {
                    Type::String
                },
                typ => Type::UnsupportedType(typ.to_owned()),
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
        let typ =
            match *self {
                Type::Nullable(box ref typ) => typ,
                ref typ => typ,
            };
        match expected_type.sty {
            TypeVariants::TyInt(IntTy::TyI32) => {
                match *typ {
                    Type::I32 | Type::Serial | Type::Custom(_) => true,
                    _ => false,
                }
            },
            TypeVariants::TyInt(IntTy::TyI64) => {
                *typ == Type::I64
            },
            TypeVariants::TyRef(_, TypeAndMut { ty, .. }) => {
                // TODO: supporter les références de références.
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
                                if let TypeVariants::TyStruct(def, _) = inner_type.sty {
                                    match def.struct_variant().name.to_string().as_str() {
                                        "UTC" => *typ == Type::UTCDateTime,
                                        "Local" => *typ == Type::LocalDateTime,
                                        _ => false,
                                    }
                                }
                                else {
                                    false
                                }
                            },
                            None => false,
                        }
                    },
                    "NaiveDate" => *typ == Type::NaiveDate,
                    "NaiveDateTime" => *typ == Type::NaiveDateTime,
                    "NaiveTime" => *typ == Type::NaiveTime,
                    _ => false,
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
fn get_type_parameter_as_path<'a>(parameters: &'a PathParameters) -> Option<&'a Path> {
    if let AngleBracketedParameters(AngleBracketedParameterData { ref types, .. }) = *parameters {
        match types.first() {
            Some(ty) => {
                if let TyPath(None, ref path) = ty.node {
                    Some(path)
                }
                else {
                    None
                }
            },
            None => None
        }
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
                let tables = singleton();
                match tables.get(related_table_name).and_then(|table| get_primary_key_field(table)) {
                    Some(primary_key_field) => "INTEGER REFERENCES ".to_owned() + &related_table_name + "(" + &primary_key_field + ")",
                    None => "".to_owned(),
                }
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
            Type::UnsupportedType(_) => "".to_owned(),
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
