use bytes::Bytes;
use multer::Field;
use predawn_core::{impl_deref, openapi::Schema};
use predawn_schema::ToSchema;
use serde::de::DeserializeOwned;

use super::ParseField;
use crate::response_error::MultipartError;

#[derive(Debug, Default, Clone, Copy)]
pub struct JsonField<T>(pub T);

impl_deref!(JsonField);

impl<T: ToSchema> ToSchema for JsonField<T> {
    fn schema() -> Schema {
        T::schema()
    }
}

impl<T: Send + DeserializeOwned> ParseField for JsonField<T> {
    async fn parse_field(
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self, MultipartError> {
        match crate::util::from_bytes(&<Bytes as ParseField>::parse_field(field, name).await?) {
            Ok(o) => Ok(JsonField(o)),
            Err(e) => Err(MultipartError::DeserializeJson { name, error: e }),
        }
    }
}
