use bytes::{BufMut, BytesMut};
use openapiv3::Schema;
use schemars::{schema::RootSchema, schema_for, JsonSchema};

pub fn schemars_transform<T: ?Sized + JsonSchema>() -> Result<Schema, serde_json::Error> {
    fn inner_transform(schema: RootSchema) -> Result<Schema, serde_json::Error> {
        let mut buf = BytesMut::with_capacity(128).writer();
        serde_json::to_writer(&mut buf, &schema)?;
        serde_json::from_slice(&buf.into_inner())
    }

    inner_transform(schema_for!(T))
}
