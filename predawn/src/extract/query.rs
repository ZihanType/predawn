use std::collections::BTreeMap;

use predawn_core::{
    api_request::ApiRequestHead,
    from_request::FromRequestHead,
    impl_deref,
    openapi::{Parameter, Schema},
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

    async fn from_request_head(head: &'a mut Head) -> Result<Self, Self::Error> {
        match crate::util::deserialize_form_from_bytes(
            head.uri.query().unwrap_or_default().as_bytes(),
        ) {
            Ok(o) => Ok(Query(o)),
            Err(e) => Err(QueryError(e)),
        }
    }
}

impl<T: ToParameters> ApiRequestHead for Query<T> {
    fn parameters(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Option<Vec<Parameter>> {
        Some(
            <T as ToParameters>::parameters(schemas, schemas_in_progress)
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
