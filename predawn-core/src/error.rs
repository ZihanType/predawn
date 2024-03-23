use std::{any, error::Error as StdError, fmt};

use http::StatusCode;

use crate::{response::Response, response_error::ResponseError};

/// Alias for a type-erased error type.
pub type BoxError = Box<dyn StdError + Send + Sync>;

#[derive(Debug)]
pub struct Error {
    response: Response,
    inner: BoxError,
    source_type_name: &'static str,
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

    pub fn downcast<T>(self) -> Result<T, Self>
    where
        T: StdError + 'static,
    {
        let Self {
            response,
            inner,
            source_type_name,
        } = self;

        match inner.downcast() {
            Ok(err) => Ok(*err),
            Err(err) => Err(Self {
                response,
                inner: err,
                source_type_name,
            }),
        }
    }

    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    pub fn response(self) -> Response {
        self.response
    }

    pub fn source_type_name(&self) -> &'static str {
        self.source_type_name
    }
}

impl<T> From<T> for Error
where
    T: ResponseError,
{
    fn from(value: T) -> Self {
        let response = value.as_response();

        Self {
            response,
            inner: value.inner(),
            source_type_name: any::type_name::<T>(),
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
                    source_type_name: any::type_name::<BoxError>(),
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
