Define a single response body with multiple media types.

This macro will generate 3 implementations, [`MultiResponseMediaType`], [`SingleResponse`] and [`IntoResponse`].

## Example

```rust
use predawn::{
    define_into_response_error,
    payload::{
        form::{Form, WriteFormError},
        json::{Json, WriteJsonError},
    },
    MultiResponseMediaType, ToSchema,
};
use serde::Serialize;

#[derive(Debug, MultiResponseMediaType)]
// `status` is optional, default is 200
#[multi_response_media_type(error = WriteJsonOrFormError, status = 200)]
pub enum JsonOrForm<T: Serialize + ToSchema> {
    Json(Json<T>),
    Form(Form<T>),
}

define_into_response_error! {
    name: WriteJsonOrFormError,
    errors: [
        WriteJsonError,
        WriteFormError,
    ],
}
```

[`MultiResponseMediaType`]: https://docs.rs/predawn/latest/predawn/trait.MultiResponseMediaType.html
[`SingleResponse`]: https://docs.rs/predawn/latest/predawn/trait.SingleResponse.html
[`IntoResponse`]: https://docs.rs/predawn/latest/predawn/into_response/trait.IntoResponse.html
