use std::{borrow::Cow, collections::BTreeMap};

use http::{
    header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    HeaderValue, StatusCode,
};
use predawn_core::{
    api_response::ApiResponse,
    either::Either,
    into_response::IntoResponse,
    media_type::{MediaType, MultiResponseMediaType, ResponseMediaType, SingleMediaType},
    openapi::{self, Schema},
    response::{MultiResponse, Response, SingleResponse},
};
use predawn_schema::ToSchema;

use crate::response_error::InvalidContentDisposition;

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
    file_name: Box<str>,
}

impl<T> Download<T> {
    pub fn inline<N>(data: T, file_name: N) -> Self
    where
        N: Into<Box<str>>,
    {
        fn inner_inline<T>(data: T, file_name: Box<str>) -> Download<T> {
            Download {
                data,
                ty: DownloadType::Inline,
                file_name,
            }
        }

        inner_inline(data, file_name.into())
    }

    pub fn attachment<N>(data: T, file_name: N) -> Self
    where
        N: Into<Box<str>>,
    {
        fn inner_attachment<T>(data: T, file_name: Box<str>) -> Download<T> {
            Download {
                data,
                ty: DownloadType::Attachment,
                file_name,
            }
        }

        inner_attachment(data, file_name.into())
    }

    fn content_disposition(
        ty: DownloadType,
        file_name: Box<str>,
    ) -> Result<HeaderValue, InvalidContentDisposition> {
        let value = format!("{}; filename=\"{}\"", ty.as_str(), file_name);

        HeaderValue::from_str(&value).map_err(|_| InvalidContentDisposition(value.into()))
    }
}

impl<T: IntoResponse + MediaType> IntoResponse for Download<T> {
    type Error = Either<T::Error, InvalidContentDisposition>;

    fn into_response(self) -> Result<Response, Self::Error> {
        let Download {
            data,
            ty,
            file_name,
        } = self;

        let mut response = data.into_response().map_err(Either::Left)?;

        let headers = response.headers_mut();

        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static(<Self as MediaType>::MEDIA_TYPE),
        );

        headers.insert(
            CONTENT_DISPOSITION,
            Self::content_disposition(ty, file_name).map_err(Either::Right)?,
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
