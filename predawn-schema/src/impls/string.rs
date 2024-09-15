use std::{
    borrow::Cow,
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    path::{Path, PathBuf},
};

use openapiv3::{Schema, SchemaData, SchemaKind, StringType, Type, VariantOrUnknownOrEmpty};

use crate::ToSchema;

macro_rules! string_impl {
    ($ty:ty) => {
        string_impl!($ty, VariantOrUnknownOrEmpty::Empty);
    };
    ($ty:ty, $format:literal) => {
        string_impl!($ty, VariantOrUnknownOrEmpty::Unknown($format.to_string()));
    };
    ($ty:ty, $format:expr) => {
        impl ToSchema for $ty {
            fn title() -> Cow<'static, str> {
                stringify!($ty).into()
            }

            fn schema(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> Schema {
                let ty = StringType {
                    format: $format,
                    ..Default::default()
                };

                Schema {
                    schema_data: SchemaData {
                        title: Some(Self::title().into()),
                        ..Default::default()
                    },
                    schema_kind: SchemaKind::Type(Type::String(ty)),
                }
            }
        }
    };
}

string_impl!(str);
string_impl!(String);
string_impl!(Path);
string_impl!(PathBuf);
string_impl!(Ipv4Addr, "ipv4");
string_impl!(Ipv6Addr, "ipv6");
string_impl!(SocketAddrV4);
string_impl!(SocketAddrV6);

macro_rules! one_of_string_impl {
    ($ty:ty; [$($elem:ty),+ $(,)?]) => {
        impl ToSchema for $ty {
            fn title() -> Cow<'static, str> {
                stringify!($ty).into()
            }

            fn schema(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Schema {
                Schema {
                    schema_data: SchemaData {
                        title: Some(Self::title().into()),
                        ..Default::default()
                    },
                    schema_kind: SchemaKind::OneOf {
                        one_of: [
                            $(
                                <$elem>::schema_ref(schemas, schemas_in_progress),
                            )+
                        ]
                        .to_vec(),
                    },
                }
            }
        }
    };
}

one_of_string_impl!(IpAddr; [Ipv4Addr, Ipv6Addr]);
one_of_string_impl!(SocketAddr; [SocketAddrV4, SocketAddrV6]);
