use std::{
    collections::{BTreeMap, HashSet},
    convert::Infallible,
    error::Error,
};

use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use mime::TEXT_PLAIN_UTF_8;

use crate::{
    error::BoxError,
    media_type::MultiResponseMediaType,
    openapi::{self, Components},
    response::Response,
};

pub trait ResponseError: Error + Send + Sync + Sized + 'static {
    fn as_status(&self) -> StatusCode;

    fn status_codes() -> HashSet<StatusCode>;

    fn as_response(&self) -> Response {
        Response::builder()
            .status(self.as_status())
            .header(
                CONTENT_TYPE,
                HeaderValue::from_static(TEXT_PLAIN_UTF_8.as_ref()),
            )
            .body(self.to_string().into())
            .unwrap()
    }

    fn responses(components: &mut Components) -> BTreeMap<StatusCode, openapi::Response> {
        Self::status_codes()
            .into_iter()
            .map(|status| {
                (
                    status,
                    openapi::Response {
                        description: status.canonical_reason().unwrap_or_default().to_string(),
                        content: <String as MultiResponseMediaType>::content(components),
                        ..Default::default()
                    },
                )
            })
            .collect()
    }

    #[doc(hidden)]
    fn inner(self) -> BoxError {
        Box::new(self)
    }

    #[doc(hidden)]
    fn wrappers(&self, errors: &mut Vec<&'static str>) {
        errors.push(std::any::type_name::<Self>());
    }
}

impl ResponseError for Infallible {
    fn as_status(&self) -> StatusCode {
        match *self {}
    }

    fn status_codes() -> HashSet<StatusCode> {
        HashSet::new()
    }

    fn as_response(&self) -> Response {
        match *self {}
    }

    fn responses(_: &mut Components) -> BTreeMap<StatusCode, openapi::Response> {
        BTreeMap::new()
    }
}
