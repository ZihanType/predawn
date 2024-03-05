use openapiv3::Schema;

use crate::ToSchema;

impl<T: ToSchema> ToSchema for Option<T> {
    const REQUIRED: bool = false;

    fn schema() -> Schema {
        let mut schema = T::schema();

        schema.schema_data.nullable = true;

        let title = schema.schema_data.title.as_deref().unwrap_or("Unknown");
        schema.schema_data.title = Some(format!("Option<{}>", title));

        schema
    }
}
