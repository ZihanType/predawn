use std::borrow::Cow;

use bytes::{BufMut, Bytes, BytesMut};
use predawn_core::openapi::{
    Schema, SchemaData, SchemaKind, StringFormat, StringType, Type, VariantOrUnknownOrEmpty,
};
use serde::{Deserialize, Serialize};
use serde_json::error::Category;
use snafu::IntoError;

use crate::response_error::{DataSnafu, DeserializeJsonError, EofSnafu, SyntaxSnafu};

pub(crate) fn deserialize_json<'de, T>(bytes: &'de [u8]) -> Result<T, DeserializeJsonError>
where
    T: Deserialize<'de>,
{
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);

    serde_path_to_error::deserialize(&mut deserializer).map_err(|err| {
        match err.inner().classify() {
            Category::Io => {
                if cfg!(debug_assertions) {
                    // we don't use `serde_json::from_reader` and instead always buffer
                    // bodies first, so we shouldn't encounter any IO errors
                    unreachable!()
                } else {
                    SyntaxSnafu.into_error(err)
                }
            }
            Category::Syntax => SyntaxSnafu.into_error(err),
            Category::Data => DataSnafu.into_error(err),
            Category::Eof => EofSnafu.into_error(err),
        }
    })
}

pub(crate) fn deserialize_form<'de, T>(
    bytes: &'de [u8],
) -> Result<T, serde_path_to_error::Error<serde_html_form::de::Error>>
where
    T: Deserialize<'de>,
{
    let deserializer = serde_html_form::Deserializer::new(form_urlencoded::parse(bytes));
    serde_path_to_error::deserialize(deserializer)
}

pub(crate) fn serialize_json<T>(
    value: &T,
) -> Result<Bytes, serde_path_to_error::Error<serde_json::Error>>
where
    T: Serialize + ?Sized,
{
    let mut writer = BytesMut::with_capacity(128).writer();

    let mut serializer = serde_json::Serializer::new(&mut writer);
    serde_path_to_error::serialize(value, &mut serializer)?;

    Ok(writer.into_inner().freeze())
}

pub(crate) fn serialize_form<T>(
    value: &T,
) -> Result<String, serde_path_to_error::Error<serde_html_form::ser::Error>>
where
    T: Serialize + ?Sized,
{
    let mut target = String::with_capacity(128);

    let mut url_encoder = form_urlencoded::Serializer::for_suffix(&mut target, 0);
    let serializer = serde_html_form::Serializer::new(&mut url_encoder);

    serde_path_to_error::serialize(value, serializer)?;
    url_encoder.finish();

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
