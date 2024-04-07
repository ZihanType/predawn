use std::{error::Error as StdError, fmt};

use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use mime::TEXT_PLAIN_UTF_8;

use crate::{response::Response, response_error::ResponseError};

/// Alias for a type-erased error type.
pub type BoxError = Box<dyn StdError + Send + Sync>;

#[derive(Debug)]
pub struct Error {
    response: Response,
    inner: BoxError,
    wrappers: Vec<&'static str>,
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
            wrappers,
        } = self;

        match inner.downcast() {
            Ok(err) => Ok((response, *err, wrappers)),
            Err(err) => Err(Self {
                response,
                inner: err,
                wrappers,
            }),
        }
    }

    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    pub fn response(self) -> Response {
        self.response
    }

    pub fn wrappers(&self) -> &[&'static str] {
        &self.wrappers
    }
}

impl<T> From<T> for Error
where
    T: ResponseError,
{
    fn from(error: T) -> Self {
        let response = error.as_response();

        let mut type_names = Vec::with_capacity(1); // at least one error
        error.wrappers(&mut type_names);

        Self {
            response,
            inner: error.inner(),
            wrappers: type_names,
        }
    }
}

impl From<(StatusCode, BoxError)> for Error {
    fn from((status, error): (StatusCode, BoxError)) -> Self {
        match error.downcast::<Self>() {
            Ok(o) => *o,
            Err(e) => {
                let response = Response::builder()
                    .status(status)
                    .header(
                        CONTENT_TYPE,
                        HeaderValue::from_static(TEXT_PLAIN_UTF_8.as_ref()),
                    )
                    .body(e.to_string().into())
                    .unwrap();

                Self {
                    response,
                    inner: e,
                    wrappers: [std::any::type_name::<BoxError>()].into(),
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
