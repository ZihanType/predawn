use std::{
    any,
    collections::{BTreeSet, HashSet},
    future::Future,
    hash::{BuildHasher, Hash},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

use bytes::Bytes;
use http::Uri;
use multer::Field;
use snafu::IntoError;

use crate::response_error::{
    ByParseFieldSnafu, DuplicateFieldSnafu, IncorrectNumberOfFieldsSnafu, MissingFieldSnafu,
    MultipartError, ParseErrorAtNameSnafu,
};

pub trait ParseField: Sized + Send {
    type Holder: Send;

    fn default_holder(name: &'static str) -> Self::Holder;

    fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> impl Future<Output = Result<Self::Holder, MultipartError>> + Send;

    fn extract(holder: Self::Holder, name: &'static str) -> Result<Self, MultipartError>;
}

impl<T: ParseField> ParseField for Option<T> {
    type Holder = T::Holder;

    fn default_holder(name: &'static str) -> Self::Holder {
        T::default_holder(name)
    }

    async fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self::Holder, MultipartError> {
        T::parse_field(holder, field, name).await
    }

    fn extract(holder: Self::Holder, name: &'static str) -> Result<Self, MultipartError> {
        match T::extract(holder, name) {
            Ok(o) => Ok(Some(o)),
            Err(MultipartError::MissingField { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl<T: ParseField> ParseField for Vec<T> {
    type Holder = Result<Self, MultipartError>;

    fn default_holder(name: &'static str) -> Self::Holder {
        MissingFieldSnafu { name }.fail()
    }

    async fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self::Holder, MultipartError> {
        let item_holder = T::parse_field(T::default_holder(name), field, name).await?;
        let item = T::extract(item_holder, name)?;

        let mut holder = holder.unwrap_or_default();
        holder.push(item);
        Ok(Ok(holder))
    }

    fn extract(holder: Self::Holder, _: &'static str) -> Result<Self, MultipartError> {
        holder
    }
}

impl<T: ParseField, const N: usize> ParseField for [T; N] {
    type Holder = Result<Vec<T>, MultipartError>;

    fn default_holder(name: &'static str) -> Self::Holder {
        MissingFieldSnafu { name }.fail()
    }

    async fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self::Holder, MultipartError> {
        let item_holder = T::parse_field(T::default_holder(name), field, name).await?;
        let item = T::extract(item_holder, name)?;

        let mut holder = holder.unwrap_or_else(|_| Vec::with_capacity(N));
        holder.push(item);
        Ok(Ok(holder))
    }

    fn extract(holder: Self::Holder, name: &'static str) -> Result<Self, MultipartError> {
        Self::try_from(holder?).map_err(|v| {
            IncorrectNumberOfFieldsSnafu {
                name,
                expected: N,
                actual: v.len(),
            }
            .build()
        })
    }
}

impl<T, S> ParseField for HashSet<T, S>
where
    T: ParseField + Hash + Eq,
    S: Send + Default + BuildHasher,
{
    type Holder = Result<Self, MultipartError>;

    fn default_holder(name: &'static str) -> Self::Holder {
        MissingFieldSnafu { name }.fail()
    }

    async fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self::Holder, MultipartError> {
        let item_holder = T::parse_field(T::default_holder(name), field, name).await?;
        let item = T::extract(item_holder, name)?;

        let mut holder = holder.unwrap_or_default();
        holder.insert(item);
        Ok(Ok(holder))
    }

    fn extract(holder: Self::Holder, _: &'static str) -> Result<Self, MultipartError> {
        holder
    }
}

impl<T: ParseField + Ord> ParseField for BTreeSet<T> {
    type Holder = Result<Self, MultipartError>;

    fn default_holder(name: &'static str) -> Self::Holder {
        MissingFieldSnafu { name }.fail()
    }

    async fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self::Holder, MultipartError> {
        let item_holder = T::parse_field(T::default_holder(name), field, name).await?;
        let item = T::extract(item_holder, name)?;

        let mut holder = holder.unwrap_or_default();
        holder.insert(item);
        Ok(Ok(holder))
    }

    fn extract(holder: Self::Holder, _: &'static str) -> Result<Self, MultipartError> {
        holder
    }
}

macro_rules! some_impl {
    ($ty:ty, $ident:ident) => {
        impl ParseField for $ty {
            type Holder = Result<Self, MultipartError>;

            fn default_holder(name: &'static str) -> Self::Holder {
                MissingFieldSnafu { name }.fail()
            }

            async fn parse_field(
                holder: Self::Holder,
                field: Field<'static>,
                name: &'static str,
            ) -> Result<Self::Holder, MultipartError> {
                if holder.is_ok() {
                    return DuplicateFieldSnafu { name }.fail();
                }

                match field.$ident().await {
                    Ok(o) => Ok(Ok(o)),
                    Err(e) => Err(ByParseFieldSnafu { name }.into_error(e)),
                }
            }

            fn extract(holder: Self::Holder, _: &'static str) -> Result<Self, MultipartError> {
                holder
            }
        }
    };
}

some_impl!(String, text);
some_impl!(Bytes, bytes);

macro_rules! some_impl_by_parse_str {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ParseField for $ty {
                type Holder = Result<Self, MultipartError>;

                fn default_holder(name: &'static str) -> Self::Holder {
                    MissingFieldSnafu { name }.fail()
                }

                async fn parse_field(
                    holder: Self::Holder,
                    field: Field<'static>,
                    name: &'static str,
                ) -> Result<Self::Holder, MultipartError> {
                    if holder.is_ok() {
                        return DuplicateFieldSnafu { name }.fail();
                    }

                    let text = <String as ParseField>::parse_field(
                        <String as ParseField>::default_holder(name),
                        field,
                        name,
                    )
                    .await??;

                    match text.parse() {
                        Ok(o) => Ok(Ok(o)),
                        Err(_) => Err(
                            ParseErrorAtNameSnafu {
                                name,
                                value: text,
                                expected_type: any::type_name::<Self>(),
                            }
                            .build()
                        ),
                    }
                }

                fn extract(holder: Self::Holder, _: &'static str) -> Result<Self, MultipartError> {
                    holder
                }
            }
        )+
    };
}

some_impl_by_parse_str![
    bool,
    char,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    f32,
    f64,
    IpAddr,
    Ipv4Addr,
    Ipv6Addr,
    SocketAddr,
    SocketAddrV4,
    SocketAddrV6,
    Uri,
];
