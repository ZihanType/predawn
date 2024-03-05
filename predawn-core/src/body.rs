use std::{
    borrow::Cow,
    convert::Infallible,
    fmt,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use futures_util::{Stream, TryStream, TryStreamExt};
use http_body::SizeHint;
use http_body_util::{Full, Limited};
use hyper::body::{Frame, Incoming};
use sync_wrapper::SyncWrapper;

use crate::error::BoxError;

pub type RequestBody = Limited<Incoming>;

pub struct ResponseBody {
    kind: Kind,
}

enum Kind {
    Single(Full<Bytes>),
    #[allow(clippy::type_complexity)]
    Stream(SyncWrapper<Pin<Box<dyn Stream<Item = Result<Bytes, BoxError>> + Send>>>),
}

impl ResponseBody {
    pub fn empty() -> Self {
        Self {
            kind: Kind::Single(Full::default()),
        }
    }

    pub fn from_stream<S>(stream: S) -> Self
    where
        S: TryStream + Send + 'static,
        S::Ok: Into<Bytes>,
        S::Error: Into<BoxError>,
    {
        Self {
            kind: Kind::Stream(SyncWrapper::new(Box::pin(
                stream.map_ok(Into::into).map_err(Into::into),
            ))),
        }
    }

    pub fn clear(&mut self) {
        *self = Self::empty();
    }
}

impl Default for ResponseBody {
    fn default() -> Self {
        Self::empty()
    }
}

impl http_body::Body for ResponseBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.get_mut().kind {
            Kind::Single(ref mut single) => Pin::new(single)
                .poll_frame(cx)
                .map_err(|a: Infallible| match a {}),
            Kind::Stream(ref mut stream) => {
                stream.get_mut().as_mut().poll_next(cx).map_ok(Frame::data)
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        match self.kind {
            Kind::Single(ref single) => single.is_end_stream(),
            Kind::Stream(_) => false,
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self.kind {
            Kind::Single(ref single) => single.size_hint(),
            Kind::Stream(_) => SizeHint::default(),
        }
    }
}

impl fmt::Debug for ResponseBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[derive(Debug)]
        struct Single;
        #[derive(Debug)]
        struct Stream;

        let mut builder = f.debug_tuple("Body");

        match self.kind {
            Kind::Single(_) => builder.field(&Single),
            Kind::Stream(_) => builder.field(&Stream),
        };

        builder.finish()
    }
}

impl From<Full<Bytes>> for ResponseBody {
    fn from(value: Full<Bytes>) -> Self {
        Self {
            kind: Kind::Single(value),
        }
    }
}

macro_rules! impl_from_by_full {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl From<$ty> for ResponseBody {
                fn from(value: $ty) -> Self {
                    ResponseBody::from(Full::from(value))
                }
            }
        )+
    };
}

impl_from_by_full![
    &'static [u8],
    Cow<'static, [u8]>,
    Vec<u8>,
    Bytes,
    &'static str,
    Cow<'static, str>,
    String,
];

impl From<Box<str>> for ResponseBody {
    fn from(value: Box<str>) -> Self {
        value.to_string().into()
    }
}

impl From<Box<[u8]>> for ResponseBody {
    fn from(value: Box<[u8]>) -> Self {
        Vec::from(value).into()
    }
}

impl From<BytesMut> for ResponseBody {
    fn from(value: BytesMut) -> Self {
        value.freeze().into()
    }
}

impl<const N: usize> From<[u8; N]> for ResponseBody {
    fn from(value: [u8; N]) -> Self {
        value.to_vec().into()
    }
}

impl<const N: usize> From<&'static [u8; N]> for ResponseBody {
    fn from(value: &'static [u8; N]) -> Self {
        value.as_slice().into()
    }
}

impl From<()> for ResponseBody {
    fn from(_: ()) -> Self {
        Self::empty()
    }
}

impl From<Infallible> for ResponseBody {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}
