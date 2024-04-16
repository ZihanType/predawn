use predawn_core::openapi::{
    Schema, SchemaData, SchemaKind, StringFormat, StringType, Type, VariantOrUnknownOrEmpty,
};
use serde::Deserialize;
use serde_json::error::Category;

use crate::response_error::DeserializeJsonError;

pub(crate) fn from_bytes<'de, T>(bytes: &'de [u8]) -> Result<T, DeserializeJsonError>
where
    T: Deserialize<'de>,
{
    let deserializer = &mut serde_json::Deserializer::from_slice(bytes);

    serde_path_to_error::deserialize(deserializer).map_err(|err| {
        match err.inner().classify() {
            Category::Io => {
                if cfg!(debug_assertions) {
                    // we don't use `serde_json::from_reader` and instead always buffer
                    // bodies first, so we shouldn't encounter any IO errors
                    unreachable!()
                } else {
                    DeserializeJsonError::SyntaxError(err)
                }
            }
            Category::Syntax => DeserializeJsonError::SyntaxError(err),
            Category::Data => DeserializeJsonError::DataError(err),
            Category::Eof => DeserializeJsonError::EofError(err),
        }
    })
}

pub(crate) fn binary_schema(title: &'static str) -> Schema {
    let ty = StringType {
        format: VariantOrUnknownOrEmpty::Item(StringFormat::Binary),
        ..Default::default()
    };

    Schema {
        schema_data: SchemaData {
            title: Some(title.to_string()),
            ..Default::default()
        },
        schema_kind: SchemaKind::Type(Type::String(ty)),
    }
}
