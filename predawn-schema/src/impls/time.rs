use std::time::{Duration, SystemTime};

use indexmap::IndexMap;
use openapiv3::{ObjectType, ReferenceOr, Schema, SchemaData, SchemaKind, Type};

use crate::ToSchema;

impl ToSchema for SystemTime {
    fn schema(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> Schema {
        let mut ty = ObjectType::default();

        ty.properties
            .insert("secs_since_epoch".to_string(), i64::schema_ref_box(schemas));
        ty.properties.insert(
            "nanos_since_epoch".to_string(),
            u32::schema_ref_box(schemas),
        );

        ty.required.push("secs_since_epoch".to_string());
        ty.required.push("nanos_since_epoch".to_string());

        Schema {
            schema_data: SchemaData {
                title: Some(stringify!(SystemTime).to_string()),
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::Object(ty)),
        }
    }
}

impl ToSchema for Duration {
    fn schema(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> Schema {
        let mut ty = ObjectType::default();

        ty.properties
            .insert("secs".to_string(), u64::schema_ref_box(schemas));
        ty.properties
            .insert("nanos".to_string(), u32::schema_ref_box(schemas));

        ty.required.push("secs".to_string());
        ty.required.push("nanos".to_string());

        Schema {
            schema_data: SchemaData {
                title: Some(stringify!(Duration).to_string()),
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::Object(ty)),
        }
    }
}
