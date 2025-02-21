use std::sync::atomic::{
    AtomicBool, AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicIsize, AtomicU8, AtomicU16,
    AtomicU32, AtomicU64, AtomicUsize,
};

use super::forward_impl;

forward_impl!(AtomicBool => bool);
forward_impl!(AtomicI8 => i8);
forward_impl!(AtomicI16 => i16);
forward_impl!(AtomicI32 => i32);
forward_impl!(AtomicI64 => i64);
forward_impl!(AtomicIsize => isize);
forward_impl!(AtomicU8 => u8);
forward_impl!(AtomicU16 => u16);
forward_impl!(AtomicU32 => u32);
forward_impl!(AtomicU64 => u64);
forward_impl!(AtomicUsize => usize);
