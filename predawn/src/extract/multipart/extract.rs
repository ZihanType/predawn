use http_body_util::BodyExt;
use mime::{FORM_DATA, MULTIPART};
use multer::Field;
use predawn_core::{
    body::RequestBody,
    from_request::{FromRequest, OptionalFromRequest},
    media_type::{MediaType, RequestMediaType, has_media_type},
    request::Head,
};
use snafu::ResultExt;

use crate::response_error::{
    ByParseMultipartSnafu, InvalidMultipartContentTypeSnafu, MultipartError,
};

#[doc(hidden)]
#[derive(Debug)]
pub struct Multipart(multer::Multipart<'static>);

impl FromRequest for Multipart {
    type Error = MultipartError;

    async fn from_request(head: &mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        let content_type = head.content_type().unwrap_or_default();

        if !<Multipart as RequestMediaType>::check_content_type(content_type) {
            return InvalidMultipartContentTypeSnafu.fail();
        }

        let boundary = multer::parse_boundary(content_type).context(ByParseMultipartSnafu)?;
        let multipart = multer::Multipart::new(body.into_data_stream(), boundary);
        Ok(Multipart(multipart))
    }
}

impl OptionalFromRequest for Multipart {
    type Error = MultipartError;

    async fn from_request(head: &mut Head, body: RequestBody) -> Result<Option<Self>, Self::Error> {
        let Some(content_type) = head.content_type() else {
            return Ok(None);
        };

        if !<Multipart as RequestMediaType>::check_content_type(content_type) {
            return InvalidMultipartContentTypeSnafu.fail();
        }

        let boundary = multer::parse_boundary(content_type).context(ByParseMultipartSnafu)?;
        let multipart = multer::Multipart::new(body.into_data_stream(), boundary);
        Ok(Some(Multipart(multipart)))
    }
}

impl Multipart {
    pub async fn next_field(&mut self) -> Result<Option<Field<'static>>, MultipartError> {
        self.0.next_field().await.context(ByParseMultipartSnafu)
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
