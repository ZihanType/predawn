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

use crate::response_error::MultipartError;

pub trait ParseField: Sized + Send {
    type Holder: Default + Send;

    fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> impl Future<Output = Result<Self::Holder, MultipartError>> + Send;

    fn extract(holder: Self::Holder, name: &'static str) -> Result<Self, MultipartError>;
}

impl<T: ParseField> ParseField for Option<T> {
    type Holder = T::Holder;

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
            Err(e) => match e {
                MultipartError::MissingField { .. } => Ok(None),
                _ => Err(e),
            },
        }
    }
}

impl<T: ParseField> ParseField for Vec<T> {
    type Holder = Option<Self>;

    async fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self::Holder, MultipartError> {
        let item_holder = T::parse_field(Default::default(), field, name).await?;
        let item = T::extract(item_holder, name)?;

        let mut holder = holder.unwrap_or_default();
        holder.push(item);
        Ok(Some(holder))
    }

    fn extract(holder: Self::Holder, name: &'static str) -> Result<Self, MultipartError> {
        holder.ok_or(MultipartError::MissingField { name })
    }
}

impl<T: ParseField, const N: usize> ParseField for [T; N] {
    type Holder = Option<Vec<T>>;

    async fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self::Holder, MultipartError> {
        let item_holder = T::parse_field(Default::default(), field, name).await?;
        let item = T::extract(item_holder, name)?;

        let mut holder = holder.unwrap_or_else(|| Vec::with_capacity(N));
        holder.push(item);
        Ok(Some(holder))
    }

    fn extract(holder: Self::Holder, name: &'static str) -> Result<Self, MultipartError> {
        let holder = holder.ok_or(MultipartError::MissingField { name })?;

        Self::try_from(holder).map_err(|v| MultipartError::IncorrectNumberOfFields {
            name,
            expected: N,
            actual: v.len(),
        })
    }
}

impl<T, S> ParseField for HashSet<T, S>
where
    T: ParseField + Hash + Eq,
    S: Send + Default + BuildHasher,
{
    type Holder = Option<Self>;

    async fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self::Holder, MultipartError> {
        let item_holder = T::parse_field(Default::default(), field, name).await?;
        let item = T::extract(item_holder, name)?;

        let mut holder = holder.unwrap_or_default();
        holder.insert(item);
        Ok(Some(holder))
    }

    fn extract(holder: Self::Holder, name: &'static str) -> Result<Self, MultipartError> {
        holder.ok_or(MultipartError::MissingField { name })
    }
}

impl<T: ParseField + Ord> ParseField for BTreeSet<T> {
    type Holder = Option<Self>;

    async fn parse_field(
        holder: Self::Holder,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self::Holder, MultipartError> {
        let item_holder = T::parse_field(Default::default(), field, name).await?;
        let item = T::extract(item_holder, name)?;

        let mut holder = holder.unwrap_or_default();
        holder.insert(item);
        Ok(Some(holder))
    }

    fn extract(holder: Self::Holder, name: &'static str) -> Result<Self, MultipartError> {
        holder.ok_or(MultipartError::MissingField { name })
    }
}

macro_rules! some_impl {
    ($ty:ty, $ident:ident) => {
        impl ParseField for $ty {
            type Holder = Option<Self>;

            async fn parse_field(
                holder: Self::Holder,
                field: Field<'static>,
                name: &'static str,
            ) -> Result<Self::Holder, MultipartError> {
                if holder.is_some() {
                    return Err(MultipartError::DuplicateField { name });
                }

                match field.$ident().await {
                    Ok(o) => Ok(Some(o)),
                    Err(e) => Err(MultipartError::ByParseField { name, error: e }),
                }
            }

            fn extract(holder: Self::Holder, name: &'static str) -> Result<Self, MultipartError> {
                holder.ok_or(MultipartError::MissingField { name })
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
                type Holder = Option<Self>;

                async fn parse_field(
                    holder: Self::Holder,
                    field: Field<'static>,
                    name: &'static str,
                ) -> Result<Self::Holder, MultipartError> {
                    if holder.is_some() {
                        return Err(MultipartError::DuplicateField { name });
                    }

                    let text = <String as ParseField>::parse_field(None, field, name)
                        .await? // <- `Ok` here must be `Some`
                        .expect("unreachable: when it is `Ok`, it must be `Some`");

                    match text.parse() {
                        Ok(o) => Ok(Some(o)),
                        Err(_) => Err(MultipartError::ParseErrorAtName {
                            name,
                            value: text.into(),
                            expected_type: any::type_name::<Self>(),
                        }),
                    }
                }

                fn extract(holder: Self::Holder, name: &'static str) -> Result<Self, MultipartError> {
                    holder.ok_or(MultipartError::MissingField { name })
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
