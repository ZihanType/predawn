use openapiv3::{Components, Schema};

use crate::ToSchema;

impl<T: ToSchema> ToSchema for Option<T> {
    const REQUIRED: bool = false;

    fn schema(components: &mut Components) -> Schema {
        let mut schema = T::schema(components);

        schema.schema_data.nullable = true;

        let title = schema.schema_data.title.as_deref().unwrap_or("Unknown");
        schema.schema_data.title = Some(format!("Option<{}>", title));

        schema
    }
}
