Define multi media type response.

This macro will generate 3 implementations, [`MultiResponseMediaType`], [`SingleResponse`] and [`IntoResponse`].

## Example

```rust
use std::collections::HashSet;

use http::StatusCode;
use predawn::{
    payload::{
        form::{Form, WriteFormError},
        json::{Json, WriteJsonError},
    },
    response_error::ResponseError,
    MultiResponseMediaType, ToSchema,
};
use serde::Serialize;

#[derive(Debug, MultiResponseMediaType)]
// `status_code` is optional, default is 200
#[multi_response_media_type(error = WriteJsonOrFormError, status_code = 200)]
pub enum JsonOrForm<T: Serialize + ToSchema> {
    Json(Json<T>),
    Form(Form<T>),
}

#[derive(Debug, thiserror::Error)]
pub enum WriteJsonOrFormError {
    #[error("{0}")]
    WriteJsonError(#[from] WriteJsonError),

    #[error("{0}")]
    WriteFormError(#[from] WriteFormError),
}

impl ResponseError for WriteJsonOrFormError {
    fn as_status(&self) -> StatusCode {
        match self {
            WriteJsonOrFormError::WriteJsonError(e) => e.as_status(),
            WriteJsonOrFormError::WriteFormError(e) => e.as_status(),
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        let mut status_codes = WriteJsonError::status_codes();
        status_codes.extend(WriteFormError::status_codes());
        status_codes
    }
}
```

[`MultiResponseMediaType`]: https://docs.rs/predawn/latest/predawn/trait.MultiResponseMediaType.html
[`SingleResponse`]: https://docs.rs/predawn/latest/predawn/trait.SingleResponse.html
[`IntoResponse`]: https://docs.rs/predawn/latest/predawn/into_response/trait.IntoResponse.html
