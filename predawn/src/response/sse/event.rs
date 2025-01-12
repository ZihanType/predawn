use std::{
    io::{self, Write},
    time::Duration,
};

use bytes::{BufMut, Bytes, BytesMut};
use serde::Serialize;
use snafu::ResultExt;

use crate::response_error::{EventStreamError, *};

#[derive(Debug, Clone)]
pub struct Event {
    ty: Option<Box<str>>,
    id: Option<Box<str>>,
    data: Option<Box<[u8]>>,
    comment: Option<Box<str>>,
    retry: Option<Duration>,
}

impl Event {
    pub(crate) fn data<T: ?Sized + Serialize>(data: &T) -> Result<Self, EventStreamError> {
        let mut buf = Vec::with_capacity(1024);
        let writer = IgnoreNewLines(&mut buf);

        let mut serializer = serde_json::Serializer::new(writer);
        serde_path_to_error::serialize(data, &mut serializer).context(SerializeJsonSnafu)?;

        Ok(Self {
            ty: Default::default(),
            id: Default::default(),
            data: Some(buf.into_boxed_slice()),
            comment: Default::default(),
            retry: Default::default(),
        })
    }

    pub(crate) fn only_comment(comment: Box<str>) -> Result<Self, EventStreamError> {
        if invalid(comment.as_bytes()) {
            return ContainNewLinesOrCarriageReturnsSnafu {
                field: InvalidSseField::Comment,
            }
            .fail();
        }

        Ok(Self {
            ty: Default::default(),
            id: Default::default(),
            data: Default::default(),
            comment: Some(comment),
            retry: Default::default(),
        })
    }

    pub fn ty<T: Into<Box<str>>>(self, ty: T) -> Result<Self, EventStreamError> {
        fn inner(mut evt: Event, ty: Box<str>) -> Result<Event, EventStreamError> {
            if invalid(ty.as_bytes()) {
                return ContainNewLinesOrCarriageReturnsSnafu {
                    field: InvalidSseField::Type,
                }
                .fail();
            }

            evt.ty = Some(ty);
            Ok(evt)
        }

        inner(self, ty.into())
    }

    pub fn id<T: Into<Box<str>>>(self, id: T) -> Result<Self, EventStreamError> {
        fn inner(mut evt: Event, id: Box<str>) -> Result<Event, EventStreamError> {
            if invalid(id.as_bytes()) {
                return ContainNewLinesOrCarriageReturnsSnafu {
                    field: InvalidSseField::Id,
                }
                .fail();
            }

            if memchr::memchr(b'\0', id.as_bytes()).is_some() {
                return IdContainNullCharacterSnafu.fail();
            }

            evt.id = Some(id);
            Ok(evt)
        }

        inner(self, id.into())
    }

    pub fn comment<T: Into<Box<str>>>(self, comment: T) -> Result<Self, EventStreamError> {
        fn inner(mut evt: Event, comment: Box<str>) -> Result<Event, EventStreamError> {
            if invalid(comment.as_bytes()) {
                return ContainNewLinesOrCarriageReturnsSnafu {
                    field: InvalidSseField::Comment,
                }
                .fail();
            }

            evt.comment = Some(comment);
            Ok(evt)
        }

        inner(self, comment.into())
    }

    pub fn retry(mut self, retry: Duration) -> Self {
        self.retry = Some(retry);
        self
    }

    pub fn as_bytes(&self) -> Bytes {
        fn append_line(buf: &mut BytesMut, key: &'static str, value: &[u8]) {
            buf.extend_from_slice(key.as_bytes());
            buf.put_u8(b':');
            buf.put_u8(b' ');
            buf.extend_from_slice(value);
            buf.put_u8(b'\n');
        }

        let Self {
            ty,
            id,
            data,
            comment,
            retry,
        } = self;

        let mut buf = BytesMut::with_capacity(1024);

        if let Some(ty) = ty {
            append_line(&mut buf, "event", ty.as_bytes());
        }

        if let Some(id) = id {
            append_line(&mut buf, "id", id.as_bytes());
        }

        if let Some(comment) = comment {
            append_line(&mut buf, "", comment.as_bytes());
        }

        if let Some(data) = data {
            append_line(&mut buf, "data", data.as_ref());
        }

        if let Some(retry) = retry {
            buf.extend_from_slice(b"retry: ");

            let retry = retry.as_millis();
            buf.extend_from_slice(retry.to_string().as_bytes());

            buf.put_u8(b'\n');
        }

        buf.put_u8(b'\n');
        buf.freeze()
    }
}

fn invalid(value: &[u8]) -> bool {
    memchr::memchr2(b'\r', b'\n', value).is_some()
}

struct IgnoreNewLines<'a>(&'a mut Vec<u8>);

impl Write for IgnoreNewLines<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut last_split = 0;

        for delimiter in memchr::memchr2_iter(b'\r', b'\n', buf) {
            self.0.write_all(&buf[last_split..delimiter])?;
            last_split = delimiter + 1;
        }

        self.0.write_all(&buf[last_split..])?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
