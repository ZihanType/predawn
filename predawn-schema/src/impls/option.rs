use std::{borrow::Cow, collections::BTreeMap};

use openapiv3::Schema;

use crate::ToSchema;

impl<T: ToSchema> ToSchema for Option<T> {
    const REQUIRED: bool = false;

    fn title() -> Cow<'static, str> {
        format!("Option<{}>", T::title()).into()
    }

    fn schema(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Schema {
        let mut schema = T::schema(schemas, schemas_in_progress);

        schema.schema_data.nullable = true;
        schema.schema_data.title = Some(Self::title().into());

        schema
    }
}
