use http_body_util::BodyExt;
use mime::{FORM_DATA, MULTIPART};
use multer::Field;
use predawn_core::{
    body::RequestBody,
    from_request::FromRequest,
    media_type::{has_media_type, MediaType, RequestMediaType},
    request::Head,
};

use crate::response_error::MultipartError;

#[doc(hidden)]
#[derive(Debug)]
pub struct Multipart(multer::Multipart<'static>);

impl<'a> FromRequest<'a> for Multipart {
    type Error = MultipartError;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        let content_type = head.content_type().unwrap_or_default();

        if <Multipart as RequestMediaType>::check_content_type(content_type) {
            let boundary =
                multer::parse_boundary(content_type).map_err(MultipartError::ByParseMultipart)?;

            let multipart = multer::Multipart::new(body.into_data_stream(), boundary);
            Ok(Multipart(multipart))
        } else {
            Err(MultipartError::InvalidMultipartContentType)
        }
    }
}

impl Multipart {
    pub async fn next_field(&mut self) -> Result<Option<Field<'static>>, MultipartError> {
        self.0
            .next_field()
            .await
            .map_err(MultipartError::ByParseMultipart)
    }
}

impl MediaType for Multipart {
    const MEDIA_TYPE: &'static str = "multipart/form-data";
}

impl RequestMediaType for Multipart {
    fn check_content_type(content_type: &str) -> bool {
        has_media_type(
            content_type,
            MULTIPART.as_str(),
            FORM_DATA.as_str(),
            FORM_DATA.as_str(),
            None,
        )
    }
}
