use http::header::CONTENT_TYPE;
pub use predawn_core::media_type::{
    assert_response_media_type, has_media_type, MediaType, RequestMediaType, ResponseMediaType,
    SingleMediaType,
};

#[derive(Debug, thiserror::Error)]
#[error("invalid `{CONTENT_TYPE}`: expected one of {expected:?} but got {actual:?}")]
pub struct InvalidContentType<const N: usize> {
    pub actual: String,
    pub expected: [&'static str; N],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_content_type() {
        let err = InvalidContentType {
            actual: "application/json".to_string(),
            expected: ["text/plain", "text/html"],
        };

        assert_eq!(
            format!("{}", err),
            "invalid `content-type`: expected one of [\"text/plain\", \"text/html\"] but got \"application/json\""
        );
    }
}
