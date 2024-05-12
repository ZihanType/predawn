use openapiv3::{Components, ReferenceOr, Schema};

pub trait ToSchema {
    const REQUIRED: bool = true;

    fn name() -> String {
        std::any::type_name::<Self>().replace("::", ".")
    }

    fn schema_ref(components: &mut Components) -> ReferenceOr<Schema> {
        let name = Self::name();

        let reference = ReferenceOr::Reference {
            reference: format!("#/components/schemas/{}", name),
        };

        if !components.schemas.contains_key(&name) {
            components
                .schemas
                .insert(name, ReferenceOr::Item(Self::schema()));
        }

        reference
    }

    fn schema() -> Schema;
}
