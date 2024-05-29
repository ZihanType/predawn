use bytes::Bytes;
use indexmap::IndexMap;
use multer::Field;
use predawn_core::openapi::{ReferenceOr, Schema};
use predawn_schema::ToSchema;

use super::ParseField;
use crate::response_error::MultipartError;

#[derive(Debug)]
pub struct Upload {
    field_name: &'static str,
    file_name: Box<str>,
    content_type: Box<str>,
    bytes: Bytes,
}

impl Upload {
    /// Return the name of the parameter in the multipart form.
    #[inline]
    pub fn field_name(&self) -> &'static str {
        self.field_name
    }

    /// Return the file name in the client's filesystem.
    #[inline]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Return the content type of the file.
    #[inline]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    #[inline]
    pub fn bytes(&self) -> &Bytes {
        &self.bytes
    }

    #[inline]
    pub fn into_bytes(self) -> Bytes {
        self.bytes
    }
}

impl ToSchema for Upload {
    fn schema(_: &mut IndexMap<String, ReferenceOr<Schema>>) -> Schema {
        crate::util::binary_schema("Upload")
    }
}

impl ParseField for Upload {
    type Holder = Option<Self>;

    async fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self::Holder, MultipartError> {
        if holder.is_some() {
            return Err(MultipartError::DuplicateField { name });
        }

        let file_name = field
            .file_name()
            .ok_or(MultipartError::MissingFileName { name })?
            .into();

        let content_type = field
            .content_type()
            .ok_or(MultipartError::MissingContentType { name })?
            .as_ref()
            .into();

        let bytes = <Bytes as ParseField>::parse_field(None, field, name)
            .await? // <- `Ok` here must be `Some`
            .expect("unreachable: when it is `Ok`, it must be `Some`");

        Ok(Some(Upload {
            field_name: name,
            file_name,
            content_type,
            bytes,
        }))
    }

    fn extract(holder: Self::Holder, name: &'static str) -> Result<Self, MultipartError> {
        holder.ok_or(MultipartError::MissingField { name })
    }
}
