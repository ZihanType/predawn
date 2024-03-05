use openapiv3::{ArrayType, ReferenceOr, Schema, SchemaData, SchemaKind, Type};

use crate::ToSchema;

macro_rules! seq_impl {
    ($($desc:tt)+) => {
        impl $($desc)+
        where
            T: ToSchema
        {
            fn schema() -> Schema {
                let elem = T::schema();
                let title = elem.schema_data.title.as_deref().unwrap_or("Unknown");
                let title = format!("Vector<{}>", title);

                let ty = ArrayType {
                    items: Some(ReferenceOr::Item(Box::new(elem))),
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
