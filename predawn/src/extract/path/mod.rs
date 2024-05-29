mod de;

use std::fmt::Display;

use indexmap::IndexMap;
use predawn_core::{
    api_request::ApiRequestHead,
    from_request::FromRequestHead,
    impl_deref,
    openapi::{Parameter, ReferenceOr, Schema},
    request::Head,
};
use serde::Deserialize;

use crate::{path_params::PathParams, response_error::PathError, ToParameters};

#[derive(Debug)]
pub struct Path<T>(pub T);

impl_deref!(Path);

impl<'a, T> FromRequestHead<'a> for Path<T>
where
    T: Deserialize<'a>,
{
    type Error = PathError;

    async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
        let params = match head.extensions.get::<PathParams>() {
            Some(PathParams::Params(params)) => params,
            Some(PathParams::InvalidUtf8InPathParam { key, error }) => {
                let err = PathError::InvalidUtf8InPathParam {
                    key: key.clone(),
                    error: *error,
                };
                return Err(err);
            }
            None => {
                return Err(PathError::MissingPathParams);
            }
        };

        T::deserialize(de::PathDeserializer::new(params)).map(Path)
    }
}

impl<T: ToParameters> ApiRequestHead for Path<T> {
    fn parameters(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> Option<Vec<Parameter>> {
        Some(
            <T as ToParameters>::parameters(schemas)
                .into_iter()
                .map(|parameter_data| Parameter::Path {
                    parameter_data,
                    style: Default::default(),
                })
                .collect(),
        )
    }
}

impl serde::de::Error for PathError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self::Message(msg.to_string())
    }
}
