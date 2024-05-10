Define a request body with `multipart/form-data` media type.

This macro will generate 5 implementations, [`FromRequest`], [`ApiRequest`], [`MediaType`], [`RequestMediaType`] and [`SingleMediaType`].

## Example

```rust
use predawn::{
    extract::multipart::{JsonField, Multipart, Upload},
    ToSchema,
};
use serde::Deserialize;

#[derive(ToSchema, Multipart)]
pub struct SomeMultipart {
    person: JsonField<Person>,
    message: String,
    files: Vec<Upload>,
}

#[derive(ToSchema, Deserialize)]
pub struct Person {
    name: String,
    age: u8,
}
```

## Note

`struct`s can only be annotated with `Multipart` derive macro if all of their fields implement the [`ParseField`] trait.

[`FromRequest`]: https://docs.rs/predawn/latest/predawn/from_request/trait.FromRequest.html
[`ApiRequest`]: https://docs.rs/predawn/latest/predawn/api_request/trait.ApiRequest.html
[`MediaType`]: https://docs.rs/predawn/latest/predawn/media_type/trait.MediaType.html
[`RequestMediaType`]: https://docs.rs/predawn/latest/predawn/media_type/trait.RequestMediaType.html
[`SingleMediaType`]: https://docs.rs/predawn/latest/predawn/media_type/trait.SingleMediaType.html
[`ParseField`]: https://docs.rs/predawn/latest/predawn/extract/multipart/trait.ParseField.html
