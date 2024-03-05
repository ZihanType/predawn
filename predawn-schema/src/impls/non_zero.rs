use std::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
};

use openapiv3::{
    AnySchema, IntegerType, ReferenceOr, Schema, SchemaData, SchemaKind, Type,
    VariantOrUnknownOrEmpty,
};

use crate::ToSchema;

macro_rules! nonzero_signed_impl {
    ($ty:ty, $format:literal) => {
        impl ToSchema for $ty {
            fn schema() -> Schema {
                Schema {
                    schema_data: SchemaData {
                        title: Some(stringify!($ty).to_string()),
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
            fn schema() -> Schema {
                Schema {
                    schema_data: SchemaData {
                        title: Some(stringify!($ty).to_string()),
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
