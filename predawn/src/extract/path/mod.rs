mod de;

use std::collections::BTreeMap;

use predawn_core::{
    api_request::ApiRequestHead,
    from_request::FromRequestHead,
    impl_deref,
    openapi::{Parameter, Schema},
    request::Head,
};
use serde::de::DeserializeOwned;
use snafu::{IntoError, ResultExt};

use self::de::PathDeserializer;
use crate::{
    ToParameters,
    path_params::PathParams,
    response_error::{PathError, path_error},
};

#[derive(Debug)]
pub struct Path<T>(pub T);

impl_deref!(Path);

impl<T> FromRequestHead for Path<T>
where
    T: DeserializeOwned,
{
    type Error = PathError;

    async fn from_request_head(head: &mut Head) -> Result<Self, Self::Error> {
        let params = match head.extensions.get::<PathParams>() {
            Some(PathParams::Ok(params)) => params,
            Some(PathParams::Err(e)) => {
                return Err(path_error::InvalidUtf8InPathParamsSnafu.into_error(e.clone()));
            }
            None => return Err(path_error::MissingPathParamsSnafu.build()),
        };

        let deserializer = PathDeserializer::new(params);

        let path = T::deserialize(deserializer).context(path_error::DeserializePathSnafu)?;
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
