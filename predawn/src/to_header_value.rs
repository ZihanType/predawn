use std::{
    borrow::Cow,
    fmt::Debug,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use bytes::Bytes;
use http::{header::InvalidHeaderValue, HeaderValue, Uri};

#[doc(hidden)]
#[inline(always)]
pub fn panic_on_err<T: ToHeaderValue>(t: &T) -> ! {
    panic!(
        "`<{} as ToHeaderValue>::to_header_value` method returns `Some(Err(_))`, the instance that called this method cannot be converted to a valid `HeaderValue`: {:?}",
        std::any::type_name::<T>(),
        t
    )
}

#[doc(hidden)]
#[inline(always)]
pub fn panic_on_none<T: ToHeaderValue>() -> ! {
    panic!(
        "`<{ty} as ToHeaderValue>::to_header_value` method returns `None`, but judging by the `<{ty} as ToSchema>::REQUIRED` constant, it should return `Some(_)`",
        ty = std::any::type_name::<T>()
    )
}

pub trait ToHeaderValue: Debug {
    fn to_header_value(&self) -> Option<Result<HeaderValue, InvalidHeaderValue>>;
}

impl<T: ToHeaderValue> ToHeaderValue for Option<T> {
    fn to_header_value(&self) -> Option<Result<HeaderValue, InvalidHeaderValue>> {
        match self {
            Some(v) => v.to_header_value(),
            None => None,
        }
    }
}

impl ToHeaderValue for bool {
    fn to_header_value(&self) -> Option<Result<HeaderValue, InvalidHeaderValue>> {
        let s = match self {
            true => "true",
            false => "false",
        };

        Some(Ok(HeaderValue::from_static(s)))
    }
}

macro_rules! impl_to_header_value_directly {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ToHeaderValue for $ty {
                fn to_header_value(&self) -> Option<Result<HeaderValue, InvalidHeaderValue>> {
                    Some(Ok(HeaderValue::from(*self)))
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
                fn to_header_value(&self) -> Option<Result<HeaderValue, InvalidHeaderValue>> {
                    Some(HeaderValue::from_str(self))
                }
            }
        )+
    };
}

impl_to_header_value_str![&'static str, Cow<'static, str>, String];

macro_rules! impl_to_header_value_bytes {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ToHeaderValue for $ty {
                fn to_header_value(&self) -> Option<Result<HeaderValue, InvalidHeaderValue>> {
                    Some(HeaderValue::from_bytes(self))
                }
            }
        )+
    };
}

impl_to_header_value_bytes![&'static [u8], Cow<'static, [u8]>, Vec<u8>, Bytes];

macro_rules! impl_to_header_value_by_to_string {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ToHeaderValue for $ty {
                fn to_header_value(&self) -> Option<Result<HeaderValue, InvalidHeaderValue>> {
                    Some(HeaderValue::try_from(self.to_string()))
                }
            }
        )+
    };
}

impl_to_header_value_by_to_string![i8, i128, u8, u128, f32, f64, Ipv4Addr, Ipv6Addr, IpAddr, Uri];
