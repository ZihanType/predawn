use std::ffi::{CStr, CString};

use super::forward_impl;

forward_impl!(CString => Vec<u8>);
forward_impl!(CStr => Vec<u8>);
