use std::collections::BTreeMap;

use openapiv3::{AdditionalProperties, ObjectType, Schema, SchemaData, SchemaKind, Type};

use crate::ToSchema;

macro_rules! map_impl {
    ($($desc:tt)+) => {
        impl $($desc)+
        where
            V: ToSchema
        {
            fn schema(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Schema {
                let schema = V::schema(schemas, schemas_in_progress);
                let title = schema.schema_data.title.as_deref().unwrap_or("Unknown");
                let title = format!("Map<String, {}>", title);

                let ty = ObjectType {
                    additional_properties: Some(AdditionalProperties::Schema(Box::new(V::schema_ref(schemas, schemas_in_progress)))),
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
