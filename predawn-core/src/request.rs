use std::{fmt, net::SocketAddr};

use http::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    request::Parts,
    Extensions, HeaderMap, HeaderValue, Method, Uri, Version,
};
use hyper::body::Incoming;
use snafu::{OptionExt, Snafu};

use crate::{body::RequestBody, impl_deref, impl_display, location::Location};

pub const DEFAULT_BODY_LIMIT: usize = 2 * 1024 * 1024; // 2 mb

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
                body_limit: BodyLimit(DEFAULT_BODY_LIMIT),
                local_addr: LocalAddr(local_addr),
                remote_addr: RemoteAddr(remote_addr),
                original_uri: OriginalUri(uri),
            },
            body,
        }
    }

    pub fn split(self) -> (Head, RequestBody) {
        let Self { head, body } = self;

        let limit = head.body_limit.0;

        (head, RequestBody::new(body, limit))
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

    pub body_limit: BodyLimit,

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
            // .field("extensions", &self.extensions)
            .field("body_limit", &self.body_limit)
            .field("local_addr", &self.local_addr)
            .field("remote_addr", &self.remote_addr)
            .field("original_uri", &self.original_uri)
            .finish()
    }
}

impl Head {
    pub fn content_type(&self) -> Option<&str> {
        self.headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
    }

    pub fn content_length(&self) -> Option<usize> {
        self.headers
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok()?.parse::<usize>().ok())
    }

    pub fn local_addr(&self) -> LocalAddr {
        self.local_addr
    }

    pub fn remote_addr(&self) -> RemoteAddr {
        self.remote_addr
    }

    pub fn original_uri(&self) -> &OriginalUri {
        &self.original_uri
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BodyLimit(pub usize);

impl_deref!(BodyLimit : usize);
impl_display!(BodyLimit);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalAddr(pub SocketAddr);

impl_deref!(LocalAddr : SocketAddr);
impl_display!(LocalAddr);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RemoteAddr(pub SocketAddr);

impl_deref!(RemoteAddr : SocketAddr);
impl_display!(RemoteAddr);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalUri(pub Uri);

impl_deref!(OriginalUri : Uri);
impl_display!(OriginalUri);

impl From<Request> for http::Request<Incoming> {
    fn from(request: Request) -> Self {
        let Request {
            head:
                Head {
                    method,
                    uri,
                    version,
                    headers,
                    extensions,
                    body_limit,
                    local_addr,
                    remote_addr,
                    original_uri,
                },
            body,
        } = request;

        let mut req = http::Request::new(body);

        *req.method_mut() = method;
        *req.uri_mut() = uri;
        *req.version_mut() = version;
        *req.headers_mut() = headers;
        *req.extensions_mut() = extensions;

        req.extensions_mut().insert(body_limit);
        req.extensions_mut().insert(local_addr);
        req.extensions_mut().insert(remote_addr);
        req.extensions_mut().insert(original_uri);

        req
    }
}

impl TryFrom<http::Request<Incoming>> for Request {
    type Error = ConvertRequestError;

    fn try_from(request: http::Request<Incoming>) -> Result<Self, Self::Error> {
        let (
            Parts {
                method,
                uri,
                version,
                headers,
                mut extensions,
                ..
            },
            body,
        ) = request.into_parts();

        let body_limit = extensions
            .remove::<BodyLimit>()
            .context(NotFoundBodyLimitSnafu)?;

        let local_addr = extensions
            .remove::<LocalAddr>()
            .context(NotFoundLocalAddrSnafu)?;

        let remote_addr = extensions
            .remove::<RemoteAddr>()
            .context(NotFoundRemoteAddrSnafu)?;

        let original_uri = extensions
            .remove::<OriginalUri>()
            .context(NotFoundOriginalUriSnafu)?;

        Ok(Self {
            head: Head {
                method,
                uri,
                version,
                headers,
                extensions,
                body_limit,
                local_addr,
                remote_addr,
                original_uri,
            },
            body,
        })
    }
}

#[derive(Debug, Snafu)]
pub enum ConvertRequestError {
    #[snafu(display("not found `body limit` in request extensions"))]
    NotFoundBodyLimit {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("not found `local address` in request extensions"))]
    NotFoundLocalAddr {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("not found `remote address` in request extensions"))]
    NotFoundRemoteAddr {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("not found `original uri` in request extensions"))]
    NotFoundOriginalUri {
        #[snafu(implicit)]
        location: Location,
    },
}
