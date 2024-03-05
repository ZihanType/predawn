use bytes::{Bytes, BytesMut};

use super::forward_impl;

forward_impl!(Bytes => Vec<u8>);
forward_impl!(BytesMut => Vec<u8>);
