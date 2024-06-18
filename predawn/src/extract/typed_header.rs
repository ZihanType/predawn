use std::collections::BTreeMap;

use headers::Header;
use predawn_core::{
    api_request::ApiRequestHead,
    from_request::FromRequestHead,
    impl_deref,
    openapi::{Parameter, Schema},
    request::Head,
};

use crate::response_error::TypedHeaderError;

#[derive(Debug, Clone, Copy, Default)]
pub struct TypedHeader<T>(pub T);

impl_deref!(TypedHeader);

impl<'a, T> FromRequestHead<'a> for TypedHeader<T>
where
    T: Header,
{
    type Error = TypedHeaderError;

    async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
        let name = T::name();

        let mut values = head.headers.get_all(name).iter();
        let is_missing = values.size_hint() == (0, Some(0));

        match T::decode(&mut values) {
            Ok(o) => Ok(Self(o)),
            Err(e) => Err(if is_missing {
                TypedHeaderError::Missing { name }
            } else {
                TypedHeaderError::DecodeError { name, error: e }
            }),
        }
    }
}

impl<T> ApiRequestHead for TypedHeader<T> {
    fn parameters(_: &mut BTreeMap<String, Schema>) -> Option<Vec<Parameter>> {
        None
    }
}
