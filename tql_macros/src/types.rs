//! SQL types.

use std::fmt::{self, Display, Formatter};

use rustc::middle::ty::{TypeAndMut, TyS, TypeVariants};
use syntax::ast::{AngleBracketedParameterData, FloatTy, IntTy, Path};
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
    I8,
    I16,
    I32,
    I64,
    LocalDateTime,
    NaiveDate,
    NaiveDateTime,
    NaiveTime,
    Serial,
    String,
    UnsupportedType(String),
    UTCDateTime,
}

impl Display for Type {
    /// Get a string representation of the SQL `Type` for display in error messages.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let typ = match *self {
            Type::Bool => "bool",
            Type::ByteString => "Vec<u8>",
            Type::Char => "char",
            Type::Custom(ref typ) => &typ[..],
            Type::F32 => "f32",
            Type::F64 => "f64",
            Type::I8 => "i8",
            Type::I16 => "i16",
            Type::I32 => "i32",
            Type::I64 => "i64",
            Type::LocalDateTime => "chrono::datetime::DateTime<chrono::offset::local::Local>",
            Type::NaiveDate => "chrono::naive::datetime::NaiveDate",
            Type::NaiveDateTime => "chrono::naive::datetime::NaiveDateTime",
            Type::NaiveTime => "chrono::naive::datetime::NaiveTime",
            Type::Serial => "i32",
            Type::String => "String",
            Type::UnsupportedType(_) => "",
            Type::UTCDateTime => "chrono::datetime::DateTime<chrono::offset::utc::UTC>",
        };
        write!(f, "{}", typ)
    }
}

impl<'a> From<&'a Path> for Type {
    fn from(&Path { ref segments, .. }: &Path) -> Type {
        let unsupported = Type::UnsupportedType("".to_owned());
        if segments.len() == 1 {
            let ident = segments[0].identifier.to_string();
            match &ident[..] {
                "DateTime" => {
                    match segments[0].parameters {
                        AngleBracketedParameters(AngleBracketedParameterData { ref types, .. }) => {
                            match types.first() {
                                Some(ty) => {
                                    if let TyPath(None, Path { ref segments, .. }) = ty.node {
                                        match segments[0].identifier.to_string().as_str() {
                                            "Local" => Type::LocalDateTime,
                                            "UTC" => Type::UTCDateTime,
                                            _ => unsupported,
                                        }
                                    }
                                    else {
                                        unsupported
                                    }
                                },
                                None => unsupported,
                            }
                        },
                        _ => unsupported,
                    }
                },
                "i32" => {
                    Type::I32
                },
                "ForeignKey" => {
                    if let AngleBracketedParameters(AngleBracketedParameterData { ref types, .. }) = segments[0].parameters {
                        match types.first() {
                            Some(ty) => {
                                if let TyPath(None, Path { ref segments, .. }) = ty.node {
                                    Type::Custom(segments[0].identifier.to_string())
                                }
                                else {
                                    unsupported // TODO
                                }
                            },
                            None => unsupported, // TODO
                        }
                    }
                    else {
                        unsupported // TODO
                    }
                },
                "NaiveDate" => Type::NaiveDate,
                "NaiveDateTime" => Type::NaiveDateTime,
                "NaiveTime" => Type::NaiveTime,
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
        match expression.node {
            ExprLit(ref literal) => {
                match literal.node {
                    LitBool(_) => *self == Type::Bool,
                    LitByte(_) => false,
                    LitByteStr(_) => *self == Type::ByteString,
                    LitChar(_) => *self == Type::Char,
                    LitFloat(_, FloatTy::TyF32) => *self == Type::F32,
                    LitFloat(_, FloatTy::TyF64) => *self == Type::F64,
                    LitFloatUnsuffixed(_) => *self == Type::F32 || *self == Type::F64,
                    LitInt(_, int_type) =>
                        match int_type {
                            SignedIntLit(IntTy::TyIs, _) => false,
                            SignedIntLit(IntTy::TyI8, _) => *self == Type::I8,
                            SignedIntLit(IntTy::TyI16, _) => *self == Type::I16,
                            SignedIntLit(IntTy::TyI32, _) => *self == Type::I32 || *self == Type::Serial,
                            SignedIntLit(IntTy::TyI64, _) => *self == Type::I64,
                            UnsignedIntLit(_) => false,
                            UnsuffixedIntLit(_) =>
                                *self == Type::I8 ||
                                *self == Type::I16 ||
                                *self == Type::I32 ||
                                *self == Type::I64 ||
                                *self == Type::Serial,
                        }
                    ,
                    LitStr(_, _) => *self == Type::String,
                }
            }
            _ => true, // Returns true, because the type checking for non-literal is done later.
        }
    }
}

impl<'tcx> PartialEq<TyS<'tcx>> for Type {
    /// Comapre the `expected_type` with `Type`.
    fn eq(&self, expected_type: &TyS<'tcx>) -> bool {
        match expected_type.sty {
            TypeVariants::TyInt(IntTy::TyI32) => {
                match *self {
                    Type::I32 | Type::Serial | Type::Custom(_) => true,
                    _ => false,
                }
            },
            TypeVariants::TyInt(IntTy::TyI64) => {
                *self == Type::I64
            },
            TypeVariants::TyRef(_, TypeAndMut { ty, .. }) => {
                // TODO: supporter les références de références.
                match ty.sty {
                    TypeVariants::TyStr => {
                        *self == Type::String
                    },
                    _ => false,
                }
            },
            TypeVariants::TyStruct(def, sub) => {
                match def.struct_variant().name.to_string().as_str() {
                    "DateTime" => {
                        match sub.types.iter().next() {
                            Some(typ) => {
                                if let TypeVariants::TyStruct(def, _) = typ.sty {
                                    match def.struct_variant().name.to_string().as_str() {
                                        "UTC" => *self == Type::UTCDateTime,
                                        "Local" => *self == Type::LocalDateTime,
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
                    //"DateTime" => *self == Type::LocalDateTime,
                    "NaiveDate" => *self == Type::NaiveDate,
                    "NaiveDateTime" => *self == Type::NaiveDateTime,
                    "NaiveTime" => *self == Type::NaiveTime,
                    _ => false,
                }
            },
            _ => false,
        }
    }
}

impl ToSql for Type {
    fn to_sql(&self) -> String {
        match *self {
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
            Type::I16 => "SMALLINT".to_owned(),
            Type::I32 => "INTEGER".to_owned(),
            Type::I64 => "BIGINT".to_owned(),
            Type::LocalDateTime => "TIMESTAMP WITH TIME ZONE".to_owned(),
            Type::NaiveDate => "DATE".to_owned(),
            Type::NaiveDateTime => "TIMESTAMP".to_owned(),
            Type::NaiveTime => "TIME".to_owned(),
            Type::Serial => "SERIAL PRIMARY KEY".to_owned(),
            Type::String => "CHARACTER VARYING".to_owned(),
            Type::UnsupportedType(_) => "".to_owned(),
            Type::UTCDateTime => "TIMESTAMP WITH TIME ZONE".to_owned(),
        }
    }
}
