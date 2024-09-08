use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime},
};

use openapiv3::{ObjectType, Schema, SchemaData, SchemaKind, Type};

use crate::ToSchema;

impl ToSchema for SystemTime {
    fn schema(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Schema {
        const SECS_SINCE_EPOCH: &str = "secs_since_epoch";
        const NANOS_SINCE_EPOCH: &str = "nanos_since_epoch";

        let mut ty = ObjectType::default();

        ty.properties.insert(
            SECS_SINCE_EPOCH.to_string(),
            i64::schema_ref_box(schemas, schemas_in_progress),
        );
        ty.properties.insert(
            NANOS_SINCE_EPOCH.to_string(),
            u32::schema_ref_box(schemas, schemas_in_progress),
        );

        ty.required.push(SECS_SINCE_EPOCH.to_string());
        ty.required.push(NANOS_SINCE_EPOCH.to_string());

        Schema {
            schema_data: SchemaData {
                title: Some("SystemTime".to_string()),
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::Object(ty)),
        }
    }
}

impl ToSchema for Duration {
    fn schema(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Schema {
        const SECS: &str = "secs";
        const NANOS: &str = "nanos";

        let mut ty = ObjectType::default();

        ty.properties.insert(
            SECS.to_string(),
            u64::schema_ref_box(schemas, schemas_in_progress),
        );
        ty.properties.insert(
            NANOS.to_string(),
            u32::schema_ref_box(schemas, schemas_in_progress),
        );

        ty.required.push(SECS.to_string());
        ty.required.push(NANOS.to_string());

        Schema {
            schema_data: SchemaData {
                title: Some("Duration".to_string()),
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::Object(ty)),
        }
    }
}
