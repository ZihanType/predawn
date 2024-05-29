use bytes::Bytes;
use indexmap::IndexMap;
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
    fn name() -> String {
        T::name()
    }

    fn schema(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> Schema {
        T::schema(schemas)
    }
}

impl<T: Send + DeserializeOwned> ParseField for JsonField<T> {
    type Holder = Option<Self>;

    async fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self::Holder, MultipartError> {
        if holder.is_some() {
            return Err(MultipartError::DuplicateField { name });
        }

        let bytes = <Bytes as ParseField>::parse_field(None, field, name)
            .await? // <- `Ok` here must be `Some`
            .expect("unreachable: when it is `Ok`, it must be `Some`");

        match crate::util::from_bytes(&bytes) {
            Ok(o) => Ok(Some(JsonField(o))),
            Err(e) => Err(MultipartError::DeserializeJson { name, error: e }),
        }
    }

    fn extract(holder: Self::Holder, name: &'static str) -> Result<Self, MultipartError> {
        holder.ok_or(MultipartError::MissingField { name })
    }
}
