use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};

use crate::response_error::EventStreamError;

#[derive(Debug, Clone)]
pub struct Event {
    ty: Option<String>,
    id: Option<String>,
    data: Option<String>,
    comment: Option<String>,
    retry: Option<Duration>,
}

impl Event {
    pub fn data<T: Into<String>>(data: T) -> Self {
        fn inner(data: String) -> Event {
            Event {
                ty: Default::default(),
                id: Default::default(),
                data: Some(data),
                comment: Default::default(),
                retry: Default::default(),
            }
        }

        inner(data.into())
    }

    pub fn ty<T: Into<String>>(&mut self, ty: T) -> &mut Self {
        fn inner(evt: &mut Event, ty: String) -> &mut Event {
            evt.ty = Some(ty);
            evt
        }

        inner(self, ty.into())
    }

    pub fn id<T: Into<String>>(&mut self, id: T) -> &mut Self {
        fn inner(evt: &mut Event, id: String) -> &mut Event {
            evt.id = Some(id);
            evt
        }

        inner(self, id.into())
    }

    pub fn comment<T: Into<String>>(&mut self, comment: T) -> &mut Self {
        fn inner(evt: &mut Event, comment: String) -> &mut Event {
            evt.comment = Some(comment);
            evt
        }

        inner(self, comment.into())
    }

    pub fn retry(&mut self, retry: Duration) -> &mut Self {
        self.retry = Some(retry);
        self
    }

    pub fn as_bytes(&self) -> Result<Bytes, EventStreamError> {
        fn append_line(buf: &mut BytesMut, key: &'static str, value: &[u8]) {
            buf.extend_from_slice(key.as_bytes());
            buf.put_u8(b':');
            buf.put_u8(b' ');
            buf.extend_from_slice(value);
            buf.put_u8(b'\n');
        }

        fn valid(value: &[u8]) -> bool {
            memchr::memchr2(b'\r', b'\n', value).is_none()
        }

        let Self {
            ty,
            id,
            data,
            comment,
            retry,
        } = self;

        let mut buf = BytesMut::new();

        if let Some(ty) = ty {
            let bytes = ty.as_bytes();

            if valid(bytes) {
                append_line(&mut buf, "event", bytes);
            } else {
                return Err(EventStreamError::invalid_type());
            }
        }

        if let Some(id) = id {
            let bytes = id.as_bytes();

            if valid(bytes) {
                append_line(&mut buf, "id", bytes);
            } else {
                return Err(EventStreamError::invalid_id());
            }
        }

        if let Some(comment) = comment {
            let bytes = comment.as_bytes();

            if valid(bytes) {
                append_line(&mut buf, "", bytes);
            } else {
                return Err(EventStreamError::invalid_comment());
            }
        }

        if let Some(data) = data {
            for line in memchr_split(b'\n', data.as_bytes()) {
                append_line(&mut buf, "data", line);
            }
        }

        if let Some(retry) = retry {
            buf.extend_from_slice(b"retry: ");

            let retry = retry.as_millis();
            buf.extend_from_slice(retry.to_string().as_bytes());

            buf.put_u8(b'\n');
        }

        buf.put_u8(b'\n');
        Ok(buf.freeze())
    }

    pub(crate) fn only_comment(comment: String) -> Self {
        Event {
            ty: Default::default(),
            id: Default::default(),
            data: Default::default(),
            comment: Some(comment),
            retry: Default::default(),
        }
    }
}

fn memchr_split(needle: u8, haystack: &[u8]) -> MemchrSplit<'_> {
    MemchrSplit {
        needle,
        haystack: Some(haystack),
    }
}

struct MemchrSplit<'a> {
    needle: u8,
    haystack: Option<&'a [u8]>,
}

impl<'a> Iterator for MemchrSplit<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let haystack = self.haystack?;
        if let Some(pos) = memchr::memchr(self.needle, haystack) {
            let (front, back) = haystack.split_at(pos);
            self.haystack = Some(&back[1..]);
            Some(front)
        } else {
            self.haystack.take()
        }
    }
}
