use std::time::{Duration, SystemTime};

use openapiv3::{ObjectType, ReferenceOr, Schema, SchemaData, SchemaKind, Type};

use crate::ToSchema;

impl ToSchema for SystemTime {
    fn schema() -> Schema {
        let mut ty = ObjectType::default();

        ty.properties.insert(
            "secs_since_epoch".to_string(),
            ReferenceOr::Item(Box::new(i64::schema())),
        );
        ty.properties.insert(
            "nanos_since_epoch".to_string(),
            ReferenceOr::Item(Box::new(u32::schema())),
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
    fn schema() -> Schema {
        let mut ty = ObjectType::default();

        ty.properties.insert(
            "secs".to_string(),
            ReferenceOr::Item(Box::new(u64::schema())),
        );
        ty.properties.insert(
            "nanos".to_string(),
            ReferenceOr::Item(Box::new(u32::schema())),
        );

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
