use openapiv3::{
    AdditionalProperties, ObjectType, ReferenceOr, Schema, SchemaData, SchemaKind, Type,
};

use crate::ToSchema;

macro_rules! map_impl {
    ($($desc:tt)+) => {
        impl $($desc)+
        where
            V: ToSchema
        {
            fn schema() -> Schema {
                let value = V::schema();
                let title = value.schema_data.title.as_deref().unwrap_or("Unknown");
                let title = format!("Map<String, {}>", title);

                let ty = ObjectType {
                    additional_properties: Some(AdditionalProperties::Schema(Box::new(ReferenceOr::Item(value)))),
                    ..Default::default()
                };

                Schema {
                    schema_data: SchemaData {
                        title: Some(title),
                        ..Default::default()
                    },
                    schema_kind: SchemaKind::Type(Type::Object(ty)),
                }
            }
        }
    };
}

map_impl!(<K, V> ToSchema for std::collections::BTreeMap<K, V>);
map_impl!(<K, V, S> ToSchema for std::collections::HashMap<K, V, S>);
map_impl!(<K, V, S> ToSchema for indexmap::map::IndexMap<K, V, S>);
