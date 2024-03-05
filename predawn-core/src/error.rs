use std::{any, error::Error as StdError, fmt};

use http::StatusCode;

use crate::{response::Response, response_error::ResponseError};

/// Alias for a type-erased error type.
pub type BoxError = Box<dyn StdError + Send + Sync>;

pub struct Error {
    as_status: fn(&Error) -> StatusCode,
    as_response: fn(&Error) -> Response,
    inner: BoxError,
    inner_type_name: &'static str,
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

    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: StdError + 'static,
    {
        self.inner.downcast_mut::<T>()
    }

    pub fn downcast<T>(self) -> Result<T, Self>
    where
        T: StdError + 'static,
    {
        let Self {
            as_status,
            as_response,
            inner,
            inner_type_name,
        } = self;

        match inner.downcast() {
            Ok(err) => Ok(*err),
            Err(err) => Err(Self {
                as_status,
                as_response,
                inner: err,
                inner_type_name,
            }),
        }
    }

    pub fn as_status(&self) -> StatusCode {
        (self.as_status)(self)
    }

    pub fn as_response(&self) -> Response {
        (self.as_response)(self)
    }

    pub fn inner_type_name(&self) -> &'static str {
        self.inner_type_name
    }
}

impl<T> From<T> for Error
where
    T: ResponseError,
{
    fn from(value: T) -> Self {
        let as_status = |err: &Error| {
            let err = err.downcast_ref::<T>().unwrap();
            err.as_status()
        };

        let as_response = |err: &Error| {
            let err = err.downcast_ref::<T>().unwrap();
            err.as_response()
        };

        Self {
            as_status,
            as_response,
            inner: Box::new(value),
            inner_type_name: any::type_name::<T>(),
        }
    }
}

impl From<BoxError> for Error {
    fn from(value: BoxError) -> Self {
        match value.downcast::<Self>() {
            Ok(o) => *o,
            Err(e) => {
                let as_status = |_: &Error| StatusCode::INTERNAL_SERVER_ERROR;

                let as_response = |_: &Error| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(().into())
                        .unwrap()
                };

                Self {
                    as_status,
                    as_response,
                    inner: e,
                    inner_type_name: any::type_name::<BoxError>(),
                }
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("response", &self.as_response())
            .field("inner", &self.inner)
            .field("inner_type_name", &self.inner_type_name)
            .finish()
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
