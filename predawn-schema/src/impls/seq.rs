use std::collections::BTreeMap;

use openapiv3::{ArrayType, Schema, SchemaData, SchemaKind, Type};

use crate::ToSchema;

macro_rules! seq_impl {
    ($($desc:tt)+) => {
        impl $($desc)+
        where
            T: ToSchema
        {
            fn schema(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Schema {
                let schema = T::schema(schemas, schemas_in_progress);
                let title = schema.schema_data.title.as_deref().unwrap_or("Unknown");
                let title = format!("List<{}>", title);

                let ty = ArrayType {
                    items: Some(T::schema_ref_box(schemas, schemas_in_progress)),
                    min_items: None,
                    max_items: None,
                    unique_items: false,
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

seq_impl!(<T> ToSchema for std::collections::BinaryHeap<T>);
seq_impl!(<T> ToSchema for std::collections::LinkedList<T>);
seq_impl!(<T> ToSchema for [T]);
seq_impl!(<T> ToSchema for Vec<T>);
seq_impl!(<T> ToSchema for std::collections::VecDeque<T>);
