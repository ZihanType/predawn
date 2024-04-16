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
    // only `<Self as ToSchema>::REQUIRED == false` should override this method.
    fn default() -> Option<Self> {
        None
    }

    fn parse_field(
        field: Field<'static>,
        name: &'static str,
    ) -> impl Future<Output = Result<Self, MultipartError>> + Send;

    fn parse_repeated_field(
        self,
        field: Field<'static>,
        name: &'static str,
    ) -> impl Future<Output = Result<Self, MultipartError>> + Send {
        let _field = field;
        async move { Err(MultipartError::RepeatedField { name }) }
    }
}

impl<T: ParseField> ParseField for Option<T> {
    fn default() -> Option<Self> {
        Some(None)
    }

    async fn parse_field(
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self, MultipartError> {
        T::parse_field(field, name).await.map(Some)
    }

    async fn parse_repeated_field(
        self,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self, MultipartError> {
        let Some(v) = self else {
            return Ok(None);
        };

        T::parse_repeated_field(v, field, name).await.map(Some)
    }
}

impl<T: ParseField> ParseField for Vec<T> {
    async fn parse_field(
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self, MultipartError> {
        let item = T::parse_field(field, name).await?;
        Ok(vec![item])
    }

    async fn parse_repeated_field(
        mut self,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self, MultipartError> {
        let item = T::parse_field(field, name).await?;
        self.push(item);
        Ok(self)
    }
}

impl<T, S> ParseField for HashSet<T, S>
where
    T: ParseField + Hash + Eq,
    S: Send + Default + BuildHasher,
{
    async fn parse_field(
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self, MultipartError> {
        let item = T::parse_field(field, name).await?;
        let mut set: HashSet<_, _> = Default::default();
        set.insert(item);
        Ok(set)
    }

    async fn parse_repeated_field(
        mut self,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self, MultipartError> {
        let item = T::parse_field(field, name).await?;
        self.insert(item);
        Ok(self)
    }
}

impl<T: ParseField + Ord> ParseField for BTreeSet<T> {
    async fn parse_field(
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self, MultipartError> {
        let item = T::parse_field(field, name).await?;
        let mut set = BTreeSet::new();
        set.insert(item);
        Ok(set)
    }

    async fn parse_repeated_field(
        mut self,
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self, MultipartError> {
        let item = T::parse_field(field, name).await?;
        self.insert(item);
        Ok(self)
    }
}

impl ParseField for String {
    async fn parse_field(
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self, MultipartError> {
        field
            .text()
            .await
            .map_err(|e| MultipartError::ByParseField { name, error: e })
    }
}

impl ParseField for Bytes {
    async fn parse_field(
        field: Field<'static>,
        name: &'static str,
    ) -> Result<Self, MultipartError> {
        field
            .bytes()
            .await
            .map_err(|e| MultipartError::ByParseField { name, error: e })
    }
}

macro_rules! impl_parse_field_by_parse_str {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ParseField for $ty {
                async fn parse_field(
                    field: Field<'static>,
                    name: &'static str,
                ) -> Result<Self, MultipartError> {
                    let text = <String as ParseField>::parse_field(field, name).await?;

                    text.parse().map_err(|_| MultipartError::ParseErrorAtName {
                        name,
                        value: text.into(),
                        expected_type: any::type_name::<$ty>(),
                    })
                }
            }
        )+
    };
}

impl_parse_field_by_parse_str![
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
