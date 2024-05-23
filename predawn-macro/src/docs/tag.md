Define an OpenAPI Tag.

This macro will generate 1 implementation, [`Tag`].

## Example

```rust
use predawn::Tag;

/// This doc will be used as the tag description
#[derive(Tag)]
#[tag(rename = "This a tag")]
pub struct Hello;
```

`rename` is optional, default is the type name.

[`Tag`]: https://docs.rs/predawn/latest/predawn/trait.Tag.html
