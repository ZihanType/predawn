pub use predawn_core::media_type::{
    has_media_type, SingleMediaType, SingleRequestMediaType, SingleResponseMediaType,
};

#[derive(Debug, thiserror::Error)]
#[error("invalid content type: expected one of {expected:?} but got {actual:?}")]
pub struct InvalidContentType {
    pub actual: String,
    pub expected: Vec<&'static str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_content_type() {
        let err = InvalidContentType {
            actual: "application/json".to_string(),
            expected: vec!["text/plain", "text/html"],
        };

        assert_eq!(
            format!("{}", err),
            "invalid content type: expected one of [\"text/plain\", \"text/html\"] but got \"application/json\""
        );
    }
}
