use openapiv3::{ArrayType, ReferenceOr, Schema, SchemaData, SchemaKind, Type};

use crate::ToSchema;

macro_rules! set_impl {
    ($($desc:tt)+) => {
        impl $($desc)+
        where
            T: ToSchema
        {
            fn schema() -> Schema {
                let elem = T::schema();
                let title = elem.schema_data.title.as_deref().unwrap_or("Unknown");
                let title = format!("Set<{}>", title);

                let ty = ArrayType {
                    items: Some(ReferenceOr::Item(Box::new(elem))),
                    min_items: None,
                    max_items: None,
                    unique_items: true,
                };

                Schema {
                    schema_data: SchemaData {
                        title: Some(title),
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
