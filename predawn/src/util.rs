use std::borrow::Cow;

use predawn_core::openapi::{
    Schema, SchemaData, SchemaKind, StringFormat, StringType, Type, VariantOrUnknownOrEmpty,
};
use serde::{Deserialize, Serialize};
use serde_json::error::Category;

use crate::response_error::DeserializeJsonError;

pub(crate) fn deserialize_json_from_bytes<'de, T>(
    bytes: &'de [u8],
) -> Result<T, DeserializeJsonError>
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

pub(crate) fn deserialize_form_from_bytes<'de, T>(
    bytes: &'de [u8],
) -> Result<T, serde_path_to_error::Error<serde_html_form::de::Error>>
where
    T: Deserialize<'de>,
{
    let deserializer = serde_html_form::Deserializer::new(form_urlencoded::parse(bytes));
    serde_path_to_error::deserialize(deserializer)
}

pub(crate) fn serialize_form_from_value<T>(
    value: &T,
) -> Result<String, serde_path_to_error::Error<serde_html_form::ser::Error>>
where
    T: Serialize,
{
    let mut target = String::new();

    let mut urlencoder = form_urlencoded::Serializer::for_suffix(&mut target, 0);
    let serializer = serde_html_form::Serializer::new(&mut urlencoder);

    serde_path_to_error::serialize(value, serializer)?;
    urlencoder.finish();

    Ok(target)
}

pub(crate) fn binary_schema(title: Cow<'static, str>) -> Schema {
    let ty = StringType {
        format: VariantOrUnknownOrEmpty::Item(StringFormat::Binary),
        ..Default::default()
    };

    Schema {
        schema_data: SchemaData {
            title: Some(title.into()),
            ..Default::default()
        },
        schema_kind: SchemaKind::Type(Type::String(ty)),
    }
}
