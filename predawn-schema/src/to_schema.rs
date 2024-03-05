use std::any;

use openapiv3::{Components, ReferenceOr, Schema};

#[doc(hidden)]
pub fn component_id<T: ?Sized>() -> String {
    any::type_name::<T>().replace("::", ".")
}

pub trait ToSchema {
    const REQUIRED: bool = true;

    fn schema_ref(components: &mut Components) -> ReferenceOr<Schema> {
        let schema_id = component_id::<Self>();

        let reference = ReferenceOr::Reference {
            reference: format!("#/components/schemas/{}", schema_id),
        };

        components
            .schemas
            .entry(schema_id)
            .or_insert_with(|| ReferenceOr::Item(Self::schema()));

        reference
    }

    fn schema() -> Schema;
}
