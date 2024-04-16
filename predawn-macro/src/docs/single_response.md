Define a single response with headers and body.

This macro will generate 3 implementations, [`SingleResponse`], [`IntoResponse`] and [`ApiResponse`].

## Example

```rust
use predawn::{payload::Json, SingleResponse, ToSchema};
use serde::Serialize;

#[derive(SingleResponse)]
// `status` is optional, default is 200
#[single_response(status = 404)]
pub struct UnitResponse;

#[derive(SingleResponse)]
pub struct TupleResponse<T: Serialize + ToSchema>(
    #[header = "X-Auth-Token"] pub String,
    // the last field, if not annotated with `#[header = "xxx"]`,
    // means that it will be the response body
    pub Json<T>,
);

// also could all fields be annotated `#[header = "xxx"]`
#[derive(SingleResponse)]
pub struct NamedResponse {
    // `AAA` will be normalized,
    // e.g. uppercase letters will be converted to lowercase letters
    #[header = "AAA"]
    pub header1: String,
    #[header = "bbb"]
    pub header2: String,
    #[header = "ccc"]
    pub header3: String,
}
```

## Note

1. Only types that implement the [`ToHeaderValue`] trait can be annotated by `#[header = "xxx"]`.

2. > All custom header names are lower cased upon conversion to a `HeaderName` value. This avoids the overhead of dynamically doing lower case conversion during the hash code computation and the comparison operation.

   Details: [HeaderName](https://docs.rs/http/latest/http/header/struct.HeaderName.html)

[`SingleResponse`]: https://docs.rs/predawn/latest/predawn/trait.SingleResponse.html
[`IntoResponse`]: https://docs.rs/predawn/latest/predawn/into_response/trait.IntoResponse.html
[`ApiResponse`]: https://docs.rs/predawn/latest/predawn/api_response/trait.ApiResponse.html
[`ToHeaderValue`]: https://docs.rs/predawn/latest/predawn/response/trait.ToHeaderValue.html
