use std::{borrow::Cow, collections::BTreeMap};

use openapiv3::{ArrayType, Schema, SchemaData, SchemaKind, Type};

use crate::ToSchema;

macro_rules! set_impl {
    ($($desc:tt)+) => {
        impl $($desc)+
        where
            T: ToSchema
        {
            fn title() -> Cow<'static, str> {
                format!("Set<{}>", T::title()).into()
            }

            fn schema(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Schema {
                let ty = ArrayType {
                    items: Some(T::schema_ref_box(schemas, schemas_in_progress)),
                    min_items: None,
                    max_items: None,
                    unique_items: true,
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
    };
}

set_impl!(<T> ToSchema for std::collections::BTreeSet<T>);
set_impl!(<T, S> ToSchema for std::collections::HashSet<T, S>);
set_impl!(<T, S> ToSchema for indexmap::set::IndexSet<T, S>);
