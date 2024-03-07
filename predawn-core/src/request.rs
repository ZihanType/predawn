use std::{fmt, net::SocketAddr};

use http::{
    header::CONTENT_TYPE, request::Parts, Extensions, HeaderMap, HeaderValue, Method, Uri, Version,
};
use http_body_util::Limited;
use hyper::body::Incoming;

use crate::{body::RequestBody, impl_deref};

pub const DEFAULT_REQUEST_BODY_LIMIT: usize = 2_097_152; // 2 mb

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequestBodyLimit(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalAddr(pub SocketAddr);

impl_deref!(LocalAddr : SocketAddr);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RemoteAddr(pub SocketAddr);

impl_deref!(RemoteAddr : SocketAddr);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalUri(pub Uri);

impl_deref!(OriginalUri : Uri);

#[derive(Debug)]
pub struct Request {
    pub head: Head,
    pub body: Incoming,
}

impl Request {
    pub fn new(
        request: http::Request<Incoming>,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> Self {
        let (
            Parts {
                method,
                uri,
                version,
                headers,
                extensions,
                ..
            },
            body,
        ) = request.into_parts();

        Self {
            head: Head {
                method,
                uri: uri.clone(),
                version,
                headers,
                extensions,
                local_addr: LocalAddr(local_addr),
                remote_addr: RemoteAddr(remote_addr),
                original_uri: OriginalUri(uri),
            },
            body,
        }
    }

    pub fn split(self) -> (Head, RequestBody) {
        let Self { head, body } = self;

        let limit = match head.extensions.get::<RequestBodyLimit>() {
            Some(RequestBodyLimit(limit)) => *limit,
            None => DEFAULT_REQUEST_BODY_LIMIT,
        };

        (head, Limited::new(body, limit))
    }
}

#[derive(Clone)]
#[non_exhaustive]
pub struct Head {
    /// The request's method
    pub method: Method,

    /// The request's URI
    pub uri: Uri,

    /// The request's version
    pub version: Version,

    /// The request's headers
    pub headers: HeaderMap<HeaderValue>,

    /// The request's extensions
    pub extensions: Extensions,

    pub(crate) local_addr: LocalAddr,

    pub(crate) remote_addr: RemoteAddr,

    pub(crate) original_uri: OriginalUri,
}

impl fmt::Debug for Head {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Head")
            .field("method", &self.method)
            .field("uri", &self.uri)
            .field("version", &self.version)
            .field("headers", &self.headers)
            .field("local_addr", &self.local_addr.0)
            .field("remote_addr", &self.remote_addr.0)
            .field("original_uri", &self.original_uri.0)
            .finish()
    }
}

impl Head {
    pub fn content_type(&self) -> Option<&str> {
        self.headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr.0
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr.0
    }

    pub fn original_uri(&self) -> &Uri {
        &self.original_uri.0
    }
}
