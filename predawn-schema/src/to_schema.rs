use openapiv3::{Components, ReferenceOr, Schema};

pub trait ToSchema {
    const REQUIRED: bool = true;

    fn name() -> String {
        std::any::type_name::<Self>().replace("::", ".")
    }

    fn schema_ref(components: &mut Components) -> ReferenceOr<Schema> {
        reference::<Self, _>(components)
    }

    fn schema_ref_box(components: &mut Components) -> ReferenceOr<Box<Schema>> {
        reference::<Self, _>(components)
    }

    fn schema(components: &mut Components) -> Schema;
}

fn reference<S: ToSchema + ?Sized, T>(components: &mut Components) -> ReferenceOr<T> {
    let name = S::name();

    let reference = ReferenceOr::Reference {
        reference: format!("#/components/schemas/{}", name),
    };

    if !components.schemas.contains_key(&name) {
        let schema = S::schema(components);
        components.schemas.insert(name, ReferenceOr::Item(schema));
    }

    reference
}
