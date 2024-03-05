use std::{
    collections::{BTreeMap, HashSet},
    convert::Infallible,
    error::Error,
};

use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use mime::TEXT_PLAIN_UTF_8;
use openapiv3::Components;

use crate::{media_type::MultiResponseMediaType, response::Response};

pub trait ResponseError: Error + Send + Sync + 'static {
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

    fn responses(components: &mut Components) -> BTreeMap<StatusCode, openapiv3::Response> {
        Self::status_codes()
            .into_iter()
            .map(|status| {
                (
                    status,
                    openapiv3::Response {
                        description: status.canonical_reason().unwrap_or_default().to_string(),
                        content: <String as MultiResponseMediaType>::content(components),
                        ..Default::default()
                    },
                )
            })
            .collect()
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

    fn responses(_: &mut Components) -> BTreeMap<StatusCode, openapiv3::Response> {
        BTreeMap::new()
    }
}
