use std::collections::BTreeMap;

use openapiv3::Schema;

use crate::ToSchema;

impl<T: ToSchema> ToSchema for Option<T> {
    const REQUIRED: bool = false;

    fn schema(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Schema {
        let mut schema = T::schema(schemas, schemas_in_progress);

        schema.schema_data.nullable = true;

        let title = schema.schema_data.title.as_deref().unwrap_or("Unknown");
        schema.schema_data.title = Some(format!("Option<{}>", title));

        schema
    }
}
