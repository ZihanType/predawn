use std::{
    borrow::Cow,
    collections::BTreeMap,
    num::{
        NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize, NonZeroU8,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
    },
};

use openapiv3::{
    AnySchema, IntegerType, ReferenceOr, Schema, SchemaData, SchemaKind, Type,
    VariantOrUnknownOrEmpty,
};

use crate::ToSchema;

macro_rules! nonzero_signed_impl {
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
                    schema_kind: SchemaKind::Any(AnySchema {
                        typ: Some("integer".to_string()),
                        format: Some($format.to_string()),
                        not: Some(Box::new(ReferenceOr::Item(Schema {
                            schema_data: SchemaData::default(),
                            schema_kind: SchemaKind::Type(Type::Integer(IntegerType {
                                format: VariantOrUnknownOrEmpty::Unknown($format.to_string()),
                                maximum: Some(0),
                                minimum: Some(0),
                                ..Default::default()
                            })),
                        }))),
                        ..Default::default()
                    }),
                }
            }
        }
    };
}

nonzero_signed_impl!(NonZeroI8, "int8");
nonzero_signed_impl!(NonZeroI16, "int16");
nonzero_signed_impl!(NonZeroI32, "int32");
nonzero_signed_impl!(NonZeroI64, "int64");
nonzero_signed_impl!(NonZeroI128, "int128");
nonzero_signed_impl!(NonZeroIsize, "int");

macro_rules! nonzero_unsigned_impl {
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
                        minimum: Some(1),
                        ..Default::default()
                    })),
                }
            }
        }
    };
}

nonzero_unsigned_impl!(NonZeroU8, "uint8");
nonzero_unsigned_impl!(NonZeroU16, "uint16");
nonzero_unsigned_impl!(NonZeroU32, "uint32");
nonzero_unsigned_impl!(NonZeroU64, "uint64");
nonzero_unsigned_impl!(NonZeroU128, "uint128");
nonzero_unsigned_impl!(NonZeroUsize, "uint");
