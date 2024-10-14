use std::{borrow::Cow, collections::BTreeMap};

use openapiv3::{
    ArrayType, BooleanType, IntegerType, NumberType, Schema, SchemaData, SchemaKind, StringType,
    Type, VariantOrUnknownOrEmpty,
};
use paste::paste;

use crate::ToSchema;

impl ToSchema for bool {
    fn title() -> Cow<'static, str> {
        "bool".into()
    }

    fn schema(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> Schema {
        Schema {
            schema_data: SchemaData {
                title: Some(Self::title().into()),
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::Boolean(BooleanType::default())),
        }
    }
}

impl ToSchema for char {
    fn title() -> Cow<'static, str> {
        "char".into()
    }

    fn schema(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> Schema {
        Schema {
            schema_data: SchemaData {
                title: Some(Self::title().into()),
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::String(StringType {
                min_length: Some(1),
                max_length: Some(1),
                ..Default::default()
            })),
        }
    }
}

macro_rules! simple_impl {
    ($ty:ty, $ty_variant:ident, $format:literal) => {
        impl ToSchema for $ty {
            fn title() -> Cow<'static, str> {
                stringify!($ty).into()
            }

            fn schema(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> Schema {
                Schema {
                    schema_data: SchemaData {
                        title: Some(Self::title().into()),
                        ..Default::default()
                    },
                    schema_kind: paste! {
                        SchemaKind::Type(Type::$ty_variant([<$ty_variant Type>] {
                            format: VariantOrUnknownOrEmpty::Unknown($format.to_string()),
                            ..Default::default()
                        }))
                    },
                }
            }
        }
    };
}

simple_impl!(f32, Number, "float");
simple_impl!(f64, Number, "double");
simple_impl!(i8, Integer, "int8");
simple_impl!(i16, Integer, "int16");
simple_impl!(i32, Integer, "int32");
simple_impl!(i64, Integer, "int64");
simple_impl!(i128, Integer, "int128");
simple_impl!(isize, Integer, "int");

macro_rules! unsigned_impl {
    ($ty:ty, $format:literal) => {
        impl ToSchema for $ty {
            fn title() -> Cow<'static, str> {
                stringify!($ty).into()
            }

            fn schema(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> Schema {
                Schema {
                    schema_data: SchemaData {
                        title: Some(Self::title().into()),
                        ..Default::default()
                    },
                    schema_kind: SchemaKind::Type(Type::Integer(IntegerType {
                        format: VariantOrUnknownOrEmpty::Unknown($format.to_string()),
                        minimum: Some(0),
                        ..Default::default()
                    })),
                }
            }
        }
    };
}

unsigned_impl!(u8, "uint8");
unsigned_impl!(u16, "uint16");
unsigned_impl!(u32, "uint32");
unsigned_impl!(u64, "uint64");
unsigned_impl!(u128, "uint128");
unsigned_impl!(usize, "uint");

impl<T: ToSchema, const N: usize> ToSchema for [T; N] {
    fn key() -> String {
        format!("Array{}<{}>", N, T::key())
    }

    fn title() -> Cow<'static, str> {
        format!("Array{}<{}>", N, T::title()).into()
    }

    fn schema(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Schema {
        let ty = ArrayType {
            items: Some(T::schema_ref_box(schemas, schemas_in_progress)),
            min_items: Some(N),
            max_items: Some(N),
            unique_items: false,
        };

        Schema {
            schema_data: SchemaData {
                title: Some(Self::title().into()),
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::Array(ty)),
        }
    }
}
