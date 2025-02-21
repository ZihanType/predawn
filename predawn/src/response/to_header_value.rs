use std::{
    borrow::Cow,
    fmt::Debug,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use bytes::Bytes;
use http::{HeaderValue, Uri};

#[derive(Debug, Clone)]
pub enum MaybeHeaderValue {
    Value(HeaderValue),
    None,
    Error,
}

pub trait ToHeaderValue: Debug {
    fn to_header_value(&self) -> MaybeHeaderValue;
}

impl<T: ToHeaderValue> ToHeaderValue for Option<T> {
    fn to_header_value(&self) -> MaybeHeaderValue {
        match self {
            Some(v) => v.to_header_value(),
            None => MaybeHeaderValue::None,
        }
    }
}

impl ToHeaderValue for bool {
    fn to_header_value(&self) -> MaybeHeaderValue {
        let s = match self {
            true => "true",
            false => "false",
        };

        MaybeHeaderValue::Value(HeaderValue::from_static(s))
    }
}

macro_rules! impl_to_header_value_directly {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ToHeaderValue for $ty {
                fn to_header_value(&self) -> MaybeHeaderValue {
                    MaybeHeaderValue::Value(HeaderValue::from(*self))
                }
            }
        )+
    };
}

impl_to_header_value_directly![i16, i32, i64, isize, u16, u32, u64, usize];

macro_rules! impl_to_header_value_str {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ToHeaderValue for $ty {
                fn to_header_value(&self) -> MaybeHeaderValue {
                    match HeaderValue::from_str(self) {
                        Ok(o) => MaybeHeaderValue::Value(o),
                        Err(_) => MaybeHeaderValue::Error,
                    }
                }
            }
        )+
    };
}

impl_to_header_value_str![&'static str, Cow<'static, str>, String, Box<str>];

macro_rules! impl_to_header_value_bytes {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ToHeaderValue for $ty {
                fn to_header_value(&self) -> MaybeHeaderValue {
                    match HeaderValue::from_bytes(self) {
                        Ok(o) => MaybeHeaderValue::Value(o),
                        Err(_) => MaybeHeaderValue::Error,
                    }
                }
            }
        )+
    };
}

impl_to_header_value_bytes![&'static [u8], Cow<'static, [u8]>, Vec<u8>, Box<[u8]>, Bytes];

macro_rules! impl_to_header_value_by_to_string {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ToHeaderValue for $ty {
                fn to_header_value(&self) -> MaybeHeaderValue {
                    match HeaderValue::try_from(self.to_string()) {
                        Ok(o) => MaybeHeaderValue::Value(o),
                        Err(_) => MaybeHeaderValue::Error,
                    }
                }
            }
        )+
    };
}

impl_to_header_value_by_to_string![
    i8, i128, u8, u128, f32, f64, Ipv4Addr, Ipv6Addr, IpAddr, Uri
];
