use std::{borrow::Cow, collections::BTreeMap};

use bytes::{BufMut, BytesMut};
use http::{
    HeaderValue, StatusCode,
    header::{CONTENT_DISPOSITION, CONTENT_TYPE},
};
use predawn_core::{
    api_response::ApiResponse,
    into_response::IntoResponse,
    media_type::{MediaType, MultiResponseMediaType, ResponseMediaType, SingleMediaType},
    openapi::{self, Schema},
    response::{MultiResponse, Response, SingleResponse},
};
use predawn_schema::ToSchema;

#[derive(Debug)]
enum DownloadType {
    Inline,
    Attachment,
}

impl DownloadType {
    fn as_str(&self) -> &'static str {
        match self {
            DownloadType::Inline => "inline",
            DownloadType::Attachment => "attachment",
        }
    }
}

#[derive(Debug)]
pub struct Download<T> {
    data: T,
    ty: DownloadType,
    file_name: HeaderValue,
}

impl<T> Download<T> {
    pub fn inline<N>(data: T, file_name: N) -> Result<Self, N::Error>
    where
        N: TryInto<HeaderValue>,
    {
        fn inner<T>(data: T, file_name: HeaderValue) -> Download<T> {
            Download {
                data,
                ty: DownloadType::Inline,
                file_name,
            }
        }

        Ok(inner(data, file_name.try_into()?))
    }

    pub fn attachment<N>(data: T, file_name: N) -> Result<Self, N::Error>
    where
        N: TryInto<HeaderValue>,
    {
        fn inner<T>(data: T, file_name: HeaderValue) -> Download<T> {
            Download {
                data,
                ty: DownloadType::Attachment,
                file_name,
            }
        }

        Ok(inner(data, file_name.try_into()?))
    }

    fn content_disposition(ty: DownloadType, file_name: HeaderValue) -> HeaderValue {
        let mut buf = BytesMut::with_capacity(16);

        buf.extend_from_slice(ty.as_str().as_bytes());
        buf.extend_from_slice(b"; filename=\"");
        buf.extend_from_slice(file_name.as_bytes());
        buf.put_u8(b'"');

        HeaderValue::from_maybe_shared(buf.freeze()).unwrap()
    }
}

impl<T: IntoResponse + MediaType> IntoResponse for Download<T> {
    type Error = T::Error;

    fn into_response(self) -> Result<Response, Self::Error> {
        let Download {
            data,
            ty,
            file_name,
        } = self;

        let mut response = data.into_response()?;

        let headers = response.headers_mut();

        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static(<Self as MediaType>::MEDIA_TYPE),
        );

        headers.insert(
            CONTENT_DISPOSITION,
            Self::content_disposition(ty, file_name),
        );

        Ok(response)
    }
}

impl<T: MediaType + ResponseMediaType> ApiResponse for Download<T> {
    fn responses(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        Some(<Self as MultiResponse>::responses(
            schemas,
            schemas_in_progress,
        ))
    }
}

impl<T> ToSchema for Download<T> {
    fn title() -> Cow<'static, str> {
        "Download".into()
    }

    fn key() -> String {
        let type_name = std::any::type_name::<Self>();

        type_name
            .find('<')
            .map_or(type_name, |lt_token| &type_name[..lt_token])
            .replace("::", ".")
    }

    fn schema(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> openapi::Schema {
        crate::util::binary_schema(Self::title())
    }
}

impl<T: MediaType> MediaType for Download<T> {
    const MEDIA_TYPE: &'static str = T::MEDIA_TYPE;
}

impl<T: ResponseMediaType> ResponseMediaType for Download<T> {}

impl<T> SingleMediaType for Download<T> {
    fn media_type(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> openapi::MediaType {
        openapi::MediaType {
            schema: Some(<Self as ToSchema>::schema_ref(schemas, schemas_in_progress)),
            ..Default::default()
        }
    }
}

impl<T: MediaType + ResponseMediaType> SingleResponse for Download<T> {
    fn response(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> openapi::Response {
        openapi::Response {
            content: <Self as MultiResponseMediaType>::content(schemas, schemas_in_progress),
            ..Default::default()
        }
    }
}
