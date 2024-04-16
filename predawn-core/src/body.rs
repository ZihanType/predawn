use std::{
    borrow::Cow,
    convert::Infallible,
    pin::Pin,
    task::{ready, Context, Poll},
};

use bytes::{Bytes, BytesMut};
use futures_util::{Stream, TryStream, TryStreamExt};
use http_body::SizeHint;
use http_body_util::{combinators::UnsyncBoxBody, BodyExt, Empty, Full, Limited, StreamBody};
use hyper::body::{Frame, Incoming};

use crate::error::BoxError;

pub type RequestBody = Limited<Incoming>;

#[derive(Debug)]
pub struct ResponseBody(UnsyncBoxBody<Bytes, BoxError>);

impl ResponseBody {
    pub fn new<B>(body: B) -> Self
    where
        B: http_body::Body<Data = Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        Self(body.map_err(Into::into).boxed_unsync())
    }

    pub fn empty() -> Self {
        Self::new(Empty::new())
    }

    pub fn from_stream<S>(stream: S) -> Self
    where
        S: TryStream + Send + 'static,
        S::Ok: Into<Bytes>,
        S::Error: Into<BoxError>,
    {
        Self::new(StreamBody::new(
            stream.map_ok(|data| Frame::data(data.into())),
        ))
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

    #[inline]
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.0).poll_frame(cx)
    }

    #[inline]
    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }

    #[inline]
    fn size_hint(&self) -> SizeHint {
        self.0.size_hint()
    }
}

impl From<Full<Bytes>> for ResponseBody {
    fn from(full: Full<Bytes>) -> Self {
        Self::new(full)
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

#[derive(Debug, Clone, Copy)]
pub struct DataStream<B> {
    body: B,
}

impl<B> DataStream<B> {
    pub fn new(body: B) -> Self {
        Self { body }
    }
}

impl<B: http_body::Body + Unpin> Stream for DataStream<B> {
    type Item = Result<B::Data, B::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match ready!(Pin::new(&mut self.body).poll_frame(cx)?) {
                Some(frame) => match frame.into_data() {
                    Ok(data) => return Poll::Ready(Some(Ok(data))),
                    Err(_) => continue,
                },
                None => return Poll::Ready(None),
            }
        }
    }
}
