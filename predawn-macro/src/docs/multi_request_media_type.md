Define a single request body with multiple media types.

This macro will generate 2 implementations, [`MultiRequestMediaType`] and [`FromRequest`].

## Example

```rust
use predawn::{
    define_from_request_error,
    payload::{
        form::{Form, ReadFormError},
        json::{Json, ReadJsonError},
    },
    MultiRequestMediaType, ToSchema,
};
use serde::de::DeserializeOwned;

#[derive(Debug, MultiRequestMediaType)]
#[multi_request_media_type(error = ReadJsonOrFormError)]
pub enum JsonOrForm<T: ToSchema + DeserializeOwned> {
    Json(Json<T>),
    Form(Form<T>),
}

define_from_request_error! {
    name: ReadJsonOrFormError,
    errors: [
        ReadJsonError,
        ReadFormError,
    ],
}
```

[`MultiRequestMediaType`]: https://docs.rs/predawn/latest/predawn/trait.MultiRequestMediaType.html
[`FromRequest`]: https://docs.rs/predawn/latest/predawn/from_request/trait.FromRequest.html
