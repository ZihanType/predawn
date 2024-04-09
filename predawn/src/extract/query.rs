use predawn_core::{
    api_request::ApiRequestHead,
    from_request::FromRequestHead,
    impl_deref,
    openapi::{Components, Parameter},
    request::Head,
};
use serde::Deserialize;

use crate::{response_error::QueryError, ToParameters};

#[derive(Debug, Clone, Copy, Default)]
pub struct Query<T>(pub T);

impl_deref!(Query);

impl<'a, T> FromRequestHead<'a> for Query<T>
where
    T: Deserialize<'a>,
{
    type Error = QueryError;

    async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
        match serde_html_form::from_str(head.uri.query().unwrap_or_default()) {
            Ok(o) => Ok(Query(o)),
            Err(e) => Err(QueryError(e)),
        }
    }
}

impl<T: ToParameters> ApiRequestHead for Query<T> {
    fn parameters(components: &mut Components) -> Option<Vec<Parameter>> {
        Some(
            <T as ToParameters>::parameters(components)
                .into_iter()
                .map(|parameter_data| Parameter::Query {
                    parameter_data,
                    allow_reserved: Default::default(),
                    style: Default::default(),
                    allow_empty_value: Default::default(),
                })
                .collect(),
        )
    }
}
