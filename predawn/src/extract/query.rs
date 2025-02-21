use std::collections::BTreeMap;

use predawn_core::{
    api_request::ApiRequestHead,
    from_request::FromRequestHead,
    impl_deref,
    openapi::{Parameter, Schema},
    request::Head,
};
use serde::de::DeserializeOwned;
use snafu::ResultExt;

use crate::{
    ToParameters,
    response_error::{QueryError, QuerySnafu},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct Query<T>(pub T);

impl_deref!(Query);

impl<T> FromRequestHead for Query<T>
where
    T: DeserializeOwned,
{
    type Error = QueryError;

    async fn from_request_head(head: &mut Head) -> Result<Self, Self::Error> {
        let bytes = head.uri.query().unwrap_or_default().as_bytes();
        let query = crate::util::deserialize_form(bytes).context(QuerySnafu)?;
        Ok(Query(query))
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
