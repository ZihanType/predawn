use std::{ops::Deref, str::Utf8Error, sync::Arc};

use matchit::Params;

#[derive(Clone, Debug)]
pub(crate) enum PathParams {
    Params(Vec<(Arc<str>, PercentDecodedStr)>),
    InvalidUtf8InPathParam { key: Arc<str>, error: Utf8Error },
}

impl Default for PathParams {
    fn default() -> Self {
        Self::Params(Default::default())
    }
}

impl PathParams {
    pub(crate) fn insert(&mut self, params: Params) {
        let PathParams::Params(current) = self else {
            return;
        };

        let params = params
            .iter()
            .map(|(k, v)| {
                let key = Arc::<str>::from(k);

                match PercentDecodedStr::new(v) {
                    Ok(decoded) => Ok((key, decoded)),
                    Err(error) => Err((key, error)),
                }
            })
            .collect::<Result<Vec<_>, _>>();

        match params {
            Ok(params) => {
                current.extend(params);
            }
            Err((key, error)) => {
                *self = PathParams::InvalidUtf8InPathParam { key, error };
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PercentDecodedStr(Arc<str>);

impl PercentDecodedStr {
    fn new(s: &str) -> Result<Self, Utf8Error> {
        percent_encoding::percent_decode_str(s)
            .decode_utf8()
            .map(|decoded| Self(decoded.as_ref().into()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn into_inner(self) -> Arc<str> {
        self.0
    }
}

impl Deref for PercentDecodedStr {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}
