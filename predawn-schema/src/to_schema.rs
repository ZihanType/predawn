use std::{borrow::Cow, collections::BTreeMap};

use openapiv3::{ReferenceOr, Schema};

pub trait ToSchema {
    const REQUIRED: bool = true;

    fn key() -> String {
        std::any::type_name::<Self>().replace("::", ".")
    }

    fn title() -> Cow<'static, str>;

    fn schema_ref(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> ReferenceOr<Schema> {
        reference::<Self, _>(schemas, schemas_in_progress)
    }

    fn schema_ref_box(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> ReferenceOr<Box<Schema>> {
        reference::<Self, _>(schemas, schemas_in_progress)
    }

    fn schema(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Schema;
}

fn reference<S, T>(
    schemas: &mut BTreeMap<String, Schema>,
    schemas_in_progress: &mut Vec<String>,
) -> ReferenceOr<T>
where
    S: ToSchema + ?Sized,
{
    let key = S::key();

    let reference = ReferenceOr::Reference {
        reference: format!("#/components/schemas/{}", key),
    };

    if !schemas.contains_key(&key) {
        // nested types
        if schemas_in_progress.contains(&key) {
            return reference;
        }

        schemas_in_progress.push(key);
        let schema = S::schema(schemas, schemas_in_progress);
        let key = schemas_in_progress.pop().expect("must have a name");

        debug_assert_eq!(key, S::key());

        schemas.insert(key, schema);
    }

    reference
}
