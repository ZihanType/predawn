Define a multiple response.

This macro will generate 3 implementations, [`MultiResponse`], [`IntoResponse`] and [`ApiResponse`].

## Example

```rust
use predawn::{
    define_into_response_error,
    payload::json::{Json, WriteJsonError},
    MultiResponse, SingleResponse, ToSchema,
};
use serde::Serialize;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorSource {
    error_code: u16,
    error_message: String,
}

#[derive(Debug, SingleResponse)]
pub struct NotFoundAccount;

#[derive(MultiResponse)]
#[multi_response(error = MultipleResponseError)]
pub enum MultipleResponse<T: ToSchema + Serialize> {
    #[status = 200]
    Ok(Json<T>),

    #[status = 404]
    NotFound(NotFoundAccount),

    #[status = 500]
    Error(Json<ErrorSource>),
}

define_into_response_error! {
    name: MultipleResponseError,
    errors: [
        WriteJsonError,
    ],
}
```

[`MultiResponse`]: https://docs.rs/predawn/latest/predawn/trait.MultiResponse.html
[`IntoResponse`]: https://docs.rs/predawn/latest/predawn/into_response/trait.IntoResponse.html
[`ApiResponse`]: https://docs.rs/predawn/latest/predawn/api_response/trait.ApiResponse.html
