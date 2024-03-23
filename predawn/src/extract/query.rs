use std::collections::HashSet;

use async_trait::async_trait;
use http::StatusCode;
use predawn_core::{
    from_request::FromRequestHead,
    impl_deref,
    openapi::{Components, Parameter},
    request::Head,
    response_error::ResponseError,
};
use serde::Deserialize;

use crate::ToParameters;

#[derive(Debug, Clone, Copy, Default)]
pub struct Query<T>(pub T);

impl_deref!(Query);

#[async_trait]
impl<'a, T> FromRequestHead<'a> for Query<T>
where
    T: Deserialize<'a> + ToParameters,
{
    type Error = QueryError;

    async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
        match serde_html_form::from_str(head.uri.query().unwrap_or_default()) {
            Ok(o) => Ok(Query(o)),
            Err(e) => Err(QueryError(e)),
        }
    }

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

#[derive(Debug, thiserror::Error)]
#[error("failed to deserialize query data: {0}")]
pub struct QueryError(#[from] serde_html_form::de::Error);

impl ResponseError for QueryError {
    fn as_status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::BAD_REQUEST].into()
    }
}
