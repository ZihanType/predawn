use indexmap::IndexMap;
use openapiv3::{ReferenceOr, Schema};

use crate::ToSchema;

impl<T: ToSchema> ToSchema for Option<T> {
    const REQUIRED: bool = false;

    fn schema(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> Schema {
        let mut schema = T::schema(schemas);

        schema.schema_data.nullable = true;

        let title = schema.schema_data.title.as_deref().unwrap_or("Unknown");
        schema.schema_data.title = Some(format!("Option<{}>", title));

        schema
    }
}
