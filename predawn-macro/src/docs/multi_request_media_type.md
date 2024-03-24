Define multi media type request.

This macro will generate 2 implementations, [`MultiRequestMediaType`] and [`FromRequest`].

## Example

```rust
use std::collections::HashSet;

use http::StatusCode;
use predawn::{
    media_type::InvalidContentType,
    payload::{
        form::{Form, ReadFormError},
        json::{Json, ReadJsonError},
    },
    response_error::ResponseError,
    MultiRequestMediaType, ToSchema,
};
use serde::de::DeserializeOwned;

#[derive(Debug, MultiRequestMediaType)]
#[multi_request_media_type(error = ReadJsonOrFormError)]
pub enum JsonOrForm<T: ToSchema + DeserializeOwned> {
    Json(Json<T>),
    Form(Form<T>),
}

#[derive(Debug, thiserror::Error)]
pub enum ReadJsonOrFormError {
    #[error("{0}")]
    ReadJsonError(#[from] ReadJsonError),

    #[error("{0}")]
    ReadFormError(#[from] ReadFormError),

    // this variant will generate `impl From<InvalidContentType> for ReadJsonOrFormError` implementation
    #[error("{0}")]
    InvalidContentType(#[from] InvalidContentType),
}

impl ResponseError for ReadJsonOrFormError {
    fn as_status(&self) -> StatusCode {
        match self {
            ReadJsonOrFormError::ReadJsonError(e) => e.as_status(),
            ReadJsonOrFormError::ReadFormError(e) => e.as_status(),
            ReadJsonOrFormError::InvalidContentType { .. } => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        let mut status_codes = ReadJsonError::status_codes();
        status_codes.extend(ReadFormError::status_codes());
        status_codes.insert(StatusCode::UNSUPPORTED_MEDIA_TYPE);
        status_codes
    }
}
```

## Note

The above example will generate the following code:

```rust ignore
impl<T> MultiRequestMediaType for JsonOrForm<T>
where
    T: ToSchema + DeserializeOwned,
{
    ...
}

#[async_trait]
impl<'a, T> FromRequest<'a> for JsonOrForm<T>
where
    T: ToSchema + DeserializeOwned,
{
    type Error = ReadJsonOrFormError;

    async fn from_request(
        head: &'a Head,
        body: RequestBody,
    ) -> Result<Self, Self::Error> {
        let content_type = head.content_type().unwrap_or_default();

        if <Json<T> as SingleRequestMediaType>::check_content_type(content_type) {
            return ...;
        }

        if <Form<T> as SingleRequestMediaType>::check_content_type(content_type) {
            return ...;
        }

        // watch this.
        Err(ReadJsonOrFormError::from(InvalidContentType {
            actual: content_type.into(),
            expected: vec![
                <Json<T> as SingleMediaType>::MEDIA_TYPE,
                <Form<T> as SingleMediaType>::MEDIA_TYPE,
            ],
        }))
    }

    ...
}
```

After all checks on `content_type` fail, the expected `content_type` is written to `InvalidContentType` and put into the custom error type, so it is required that the custom error type must implement the `From<InvalidContentType>` trait.

[`MultiRequestMediaType`]: https://docs.rs/predawn/latest/predawn/trait.MultiRequestMediaType.html
[`FromRequest`]: https://docs.rs/predawn/latest/predawn/from_request/trait.FromRequest.html
