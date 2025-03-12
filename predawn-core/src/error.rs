use std::{error::Error as StdError, fmt};

use error2::Location;
use http::{HeaderValue, StatusCode, header::CONTENT_TYPE};
use mime::TEXT_PLAIN_UTF_8;

use crate::{response::Response, response_error::ResponseError};

/// Alias for a type-erased error type.
pub type BoxError = Box<dyn StdError + Send + Sync>;

#[derive(Debug)]
pub struct Error {
    response: Response,
    inner: BoxError,
    error_stack: Box<[Box<str>]>,
}

impl Error {
    pub fn is<T>(&self) -> bool
    where
        T: StdError + 'static,
    {
        self.inner.is::<T>()
    }

    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: StdError + 'static,
    {
        self.inner.downcast_ref::<T>()
    }

    #[allow(clippy::type_complexity)]
    #[allow(clippy::result_large_err)]
    pub fn downcast<T>(self) -> Result<(Response, T, Box<[Box<str>]>), Self>
    where
        T: StdError + 'static,
    {
        let Self {
            response,
            inner,
            error_stack,
        } = self;

        match inner.downcast::<T>() {
            Ok(err) => Ok((response, *err, error_stack)),
            Err(err) => Err(Self {
                response,
                inner: err,
                error_stack,
            }),
        }
    }

    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    pub fn response(self) -> Response {
        self.response
    }

    pub fn error_stack(&self) -> &[Box<str>] {
        &self.error_stack
    }
}

impl<T> From<T> for Error
where
    T: ResponseError,
{
    fn from(error: T) -> Self {
        let response = error.as_response();
        let error_stack = error.error_stack();

        let inner = error.inner();

        Self {
            response,
            inner,
            error_stack,
        }
    }
}

impl From<(StatusCode, BoxError)> for Error {
    #[track_caller]
    fn from((status, mut error): (StatusCode, BoxError)) -> Self {
        loop {
            match error.downcast::<Error>() {
                Ok(o) => {
                    if o.inner.is::<Error>() {
                        error = o.inner;
                    } else {
                        return *o;
                    }
                }
                Err(e) => {
                    error = e;
                    break;
                }
            }
        }

        let response = Response::builder()
            .status(status)
            .header(
                CONTENT_TYPE,
                HeaderValue::from_static(TEXT_PLAIN_UTF_8.as_ref()),
            )
            .body(error.to_string().into())
            .unwrap();

        let mut stack = Vec::new();
        stack.push(format!("0: {}, at: {}", error, Location::caller()).into_boxed_str());

        Self {
            response,
            inner: error,
            error_stack: stack.into_boxed_slice(),
        }
    }
}

impl From<BoxError> for Error {
    #[track_caller]
    #[inline]
    fn from(error: BoxError) -> Self {
        Error::from((StatusCode::INTERNAL_SERVER_ERROR, error))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&*self.inner)
    }
}

impl AsRef<dyn StdError + Send + Sync> for Error {
    fn as_ref(&self) -> &(dyn StdError + Send + Sync + 'static) {
        &*self.inner
    }
}

impl AsRef<dyn StdError> for Error {
    fn as_ref(&self) -> &(dyn StdError + 'static) {
        &*self.inner
    }
}
