use std::collections::BTreeMap;

use headers::Header;
use predawn_core::{
    api_request::ApiRequestHead,
    from_request::{FromRequestHead, OptionalFromRequestHead},
    impl_deref,
    openapi::{Parameter, Schema},
    request::Head,
};
use snafu::IntoError;

use crate::response_error::{DecodeSnafu, MissingSnafu, TypedHeaderError};

#[derive(Debug, Clone, Copy, Default)]
pub struct TypedHeader<T>(pub T);

impl_deref!(TypedHeader);

impl<T> FromRequestHead for TypedHeader<T>
where
    T: Header,
{
    type Error = TypedHeaderError;

    async fn from_request_head(head: &mut Head) -> Result<Self, Self::Error> {
        let name = T::name();

        let mut values = head.headers.get_all(name).iter();
        let is_missing = values.size_hint() == (0, Some(0));

        match T::decode(&mut values) {
            Ok(o) => Ok(Self(o)),
            Err(e) => Err(if is_missing {
                MissingSnafu { name }.build()
            } else {
                DecodeSnafu { name }.into_error(e)
            }),
        }
    }
}

impl<T> OptionalFromRequestHead for TypedHeader<T>
where
    T: Header,
{
    type Error = TypedHeaderError;

    async fn from_request_head(head: &mut Head) -> Result<Option<Self>, Self::Error> {
        let name = T::name();

        let mut values = head.headers.get_all(name).iter();
        let is_missing = values.size_hint() == (0, Some(0));

        match T::decode(&mut values) {
            Ok(o) => Ok(Some(Self(o))),
            Err(_) if is_missing => Ok(None),
            Err(e) => Err(DecodeSnafu { name }.into_error(e)),
        }
    }
}

impl<T> ApiRequestHead for TypedHeader<T> {
    fn parameters(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> Option<Vec<Parameter>> {
        None
    }
}
