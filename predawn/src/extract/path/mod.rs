mod de;

use std::collections::BTreeMap;

use predawn_core::{
    api_request::ApiRequestHead,
    from_request::FromRequestHead,
    impl_deref,
    openapi::{Parameter, Schema},
    request::Head,
};
use serde::Deserialize;
use snafu::ResultExt;

use self::de::PathDeserializer;
use crate::{
    path_params::PathParams,
    response_error::{DeserializePathSnafu, MissingPathParamsSnafu, PathError},
    ToParameters,
};

#[derive(Debug)]
pub struct Path<T>(pub T);

impl_deref!(Path);

impl<'a, T> FromRequestHead<'a> for Path<T>
where
    T: Deserialize<'a>,
{
    type Error = PathError;

    async fn from_request_head(head: &'a mut Head) -> Result<Self, Self::Error> {
        let params = match head.extensions.get::<PathParams>() {
            Some(PathParams(params)) => params,
            None => return MissingPathParamsSnafu.fail(),
        };

        let deserializer = PathDeserializer::new(params);

        let path = T::deserialize(deserializer).context(DeserializePathSnafu)?;
        Ok(Path(path))
    }
}

impl<T: ToParameters> ApiRequestHead for Path<T> {
    fn parameters(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Option<Vec<Parameter>> {
        Some(
            <T as ToParameters>::parameters(schemas, schemas_in_progress)
                .into_iter()
                .map(|parameter_data| Parameter::Path {
                    parameter_data,
                    style: Default::default(),
                })
                .collect(),
        )
    }
}
