Define an OpenAPI Security Scheme.

This macro will generate 1 implementation, [`SecurityScheme`].

## Example

```rust
use predawn::SecurityScheme;

/// This doc will be used as the tag description
#[derive(SecurityScheme)]
#[api_key(in = header, name = "X-API-Key")]
pub struct ApiKeyScheme;

#[derive(SecurityScheme)]
#[http(scheme = basic, rename = "Basic Auth")]
pub struct HttpScheme;
```

`rename` is optional, default is the type name.

[`SecurityScheme`]: https://docs.rs/predawn/latest/predawn/trait.SecurityScheme.html
