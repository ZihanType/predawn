use std::collections::BTreeMap;

use openapiv3::{ReferenceOr, Schema};

pub trait ToSchema {
    const REQUIRED: bool = true;

    fn name() -> String {
        std::any::type_name::<Self>().replace("::", ".")
    }

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
    let name = S::name();

    let reference = ReferenceOr::Reference {
        reference: format!("#/components/schemas/{}", name),
    };

    if !schemas.contains_key(&name) {
        // nested types
        if schemas_in_progress.contains(&name) {
            return reference;
        }

        schemas_in_progress.push(name);
        let schema = S::schema(schemas, schemas_in_progress);
        let name = schemas_in_progress.pop().expect("must have a name");

        debug_assert_eq!(name, S::name());

        schemas.insert(name, schema);
    }

    reference
}
