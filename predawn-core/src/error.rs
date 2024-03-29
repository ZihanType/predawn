use std::{any, error::Error as StdError, fmt};

use http::StatusCode;

use crate::{response::Response, response_error::ResponseError};

/// Alias for a type-erased error type.
pub type BoxError = Box<dyn StdError + Send + Sync>;

#[derive(Debug)]
pub struct Error {
    response: Response,
    inner: BoxError,
    error_wrappers: Vec<&'static str>,
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

    pub fn downcast<T>(self) -> Result<(Response, T, Vec<&'static str>), Self>
    where
        T: StdError + 'static,
    {
        let Self {
            response,
            inner,
            error_wrappers,
        } = self;

        match inner.downcast() {
            Ok(err) => Ok((response, *err, error_wrappers)),
            Err(err) => Err(Self {
                response,
                inner: err,
                error_wrappers,
            }),
        }
    }

    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    pub fn response(self) -> Response {
        self.response
    }

    pub fn error_wrappers(&self) -> &[&'static str] {
        &self.error_wrappers
    }
}

impl<T> From<T> for Error
where
    T: ResponseError,
{
    fn from(value: T) -> Self {
        let response = value.as_response();

        let mut error_wrappers = Vec::with_capacity(1); // at least one error
        value.wrappers(&mut error_wrappers);

        Self {
            response,
            inner: value.inner(),
            error_wrappers,
        }
    }
}

impl From<(StatusCode, BoxError)> for Error {
    fn from((status, error): (StatusCode, BoxError)) -> Self {
        match error.downcast::<Self>() {
            Ok(o) => *o,
            Err(e) => {
                let response = Response::builder().status(status).body(().into()).unwrap();

                Self {
                    response,
                    inner: e,
                    error_wrappers: [any::type_name::<BoxError>()].into(),
                }
            }
        }
    }
}

impl From<BoxError> for Error {
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
