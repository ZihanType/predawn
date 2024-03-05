use std::time::{Duration, SystemTime};

use openapiv3::{ObjectType, ReferenceOr, Schema, SchemaData, SchemaKind, Type};

use crate::ToSchema;

impl ToSchema for SystemTime {
    fn schema() -> Schema {
        let mut ty = ObjectType::default();

        ty.properties.insert(
            "seconds_since_epoch".to_string(),
            ReferenceOr::Item(Box::new(i64::schema())),
        );
        ty.properties.insert(
            "nanoseconds_since_epoch".to_string(),
            ReferenceOr::Item(Box::new(u32::schema())),
        );

        ty.required.push("seconds_since_epoch".to_string());
        ty.required.push("nanoseconds_since_epoch".to_string());

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
            "seconds".to_string(),
            ReferenceOr::Item(Box::new(u64::schema())),
        );
        ty.properties.insert(
            "nanoseconds".to_string(),
            ReferenceOr::Item(Box::new(u32::schema())),
        );

        ty.required.push("seconds".to_string());
        ty.required.push("nanoseconds".to_string());

        Schema {
            schema_data: SchemaData {
                title: Some(stringify!(Duration).to_string()),
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::Object(ty)),
        }
    }
}
