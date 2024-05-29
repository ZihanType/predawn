use indexmap::IndexMap;
use openapiv3::{ReferenceOr, Schema};

pub trait ToSchema {
    const REQUIRED: bool = true;

    fn name() -> String {
        std::any::type_name::<Self>().replace("::", ".")
    }

    fn schema_ref(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> ReferenceOr<Schema> {
        reference::<Self, _>(schemas)
    }

    fn schema_ref_box(
        schemas: &mut IndexMap<String, ReferenceOr<Schema>>,
    ) -> ReferenceOr<Box<Schema>> {
        reference::<Self, _>(schemas)
    }

    fn schema(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> Schema;
}

fn reference<S, T>(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> ReferenceOr<T>
where
    S: ToSchema + ?Sized,
{
    let name = S::name();

    let reference = ReferenceOr::Reference {
        reference: format!("#/components/schemas/{}", name),
    };

    if !schemas.contains_key(&name) {
        let schema = S::schema(schemas);
        schemas.insert(name, ReferenceOr::Item(schema));
    }

    reference
}
