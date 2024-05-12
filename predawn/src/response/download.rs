use std::collections::BTreeMap;

use http::{
    header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    HeaderValue, StatusCode,
};
use predawn_core::{
    api_response::ApiResponse,
    into_response::IntoResponse,
    media_type::{MediaType, MultiResponseMediaType, ResponseMediaType, SingleMediaType},
    openapi::{self, Components},
    response::{MultiResponse, Response, SingleResponse},
};
use predawn_schema::ToSchema;

use crate::response_error::DownloadError;

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

    fn content_disposition<E>(
        ty: DownloadType,
        file_name: Box<str>,
    ) -> Result<HeaderValue, DownloadError<E>> {
        let content_disposition = format!("{}; filename=\"{}\"", ty.as_str(), file_name);

        HeaderValue::from_str(&content_disposition).map_err(|_| {
            DownloadError::InvalidContentDisposition {
                value: content_disposition.into(),
            }
        })
    }
}

impl<T: IntoResponse + MediaType> IntoResponse for Download<T> {
    type Error = DownloadError<T::Error>;

    fn into_response(self) -> Result<Response, Self::Error> {
        let Download {
            data,
            ty,
            file_name,
        } = self;

        let mut response = data.into_response().map_err(DownloadError::Inner)?;

        let headers = response.headers_mut();

        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static(<Self as MediaType>::MEDIA_TYPE),
        );

        headers.insert(
            CONTENT_DISPOSITION,
            Self::content_disposition::<T::Error>(ty, file_name)?,
        );

        Ok(response)
    }
}

impl<T: MediaType + ResponseMediaType> ApiResponse for Download<T> {
    fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        Some(<Self as MultiResponse>::responses(components))
    }
}

impl<T> ToSchema for Download<T> {
    fn name() -> String {
        let type_name = std::any::type_name::<Self>();

        type_name
            .find('<')
            .map_or(type_name, |end| &type_name[..end])
            .replace("::", ".")
            .to_string()
    }

    fn schema() -> openapi::Schema {
        crate::util::binary_schema("Download")
    }
}

impl<T: MediaType> MediaType for Download<T> {
    const MEDIA_TYPE: &'static str = T::MEDIA_TYPE;
}

impl<T: ResponseMediaType> ResponseMediaType for Download<T> {}

impl<T> SingleMediaType for Download<T> {
    fn media_type(components: &mut Components) -> openapi::MediaType {
        openapi::MediaType {
            schema: Some(<Self as ToSchema>::schema_ref(components)),
            ..Default::default()
        }
    }
}

impl<T: MediaType + ResponseMediaType> SingleResponse for Download<T> {
    fn response(components: &mut Components) -> openapi::Response {
        openapi::Response {
            content: <Self as MultiResponseMediaType>::content(components),
            ..Default::default()
        }
    }
}
