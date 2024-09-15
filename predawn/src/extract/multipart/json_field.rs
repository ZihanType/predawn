use std::{borrow::Cow, collections::BTreeMap};

use bytes::Bytes;
use multer::Field;
use predawn_core::{
    impl_deref,
    openapi::{ReferenceOr, Schema},
};
use predawn_schema::ToSchema;
use serde::de::DeserializeOwned;

use super::ParseField;
use crate::response_error::MultipartError;

#[derive(Debug, Default, Clone, Copy)]
pub struct JsonField<T>(pub T);

impl_deref!(JsonField);

impl<T: ToSchema> ToSchema for JsonField<T> {
    const REQUIRED: bool = T::REQUIRED;

    fn key() -> String {
        T::key()
    }

    fn title() -> Cow<'static, str> {
        T::title()
    }

    fn schema_ref(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> ReferenceOr<Schema> {
        T::schema_ref(schemas, schemas_in_progress)
    }

    fn schema_ref_box(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> ReferenceOr<Box<Schema>> {
        T::schema_ref_box(schemas, schemas_in_progress)
    }

    fn schema(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Schema {
        T::schema(schemas, schemas_in_progress)
    }
}

impl<T: Send + DeserializeOwned> ParseField for JsonField<T> {
    type Holder = Result<Self, MultipartError>;

    fn default_holder(name: &'static str) -> Self::Holder {
        Err(MultipartError::MissingField { name })
    }

    async fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self::Holder, MultipartError> {
        if holder.is_ok() {
            return Err(MultipartError::DuplicateField { name });
        }

        let bytes = <Bytes as ParseField>::parse_field(
            <Bytes as ParseField>::default_holder(name),
            field,
            name,
        )
        .await??;

        match crate::util::deserialize_json_from_bytes(&bytes) {
            Ok(o) => Ok(Ok(JsonField(o))),
            Err(e) => Err(MultipartError::DeserializeJson { name, error: e }),
        }
    }

    fn extract(holder: Self::Holder, _: &'static str) -> Result<Self, MultipartError> {
        holder
    }
}
