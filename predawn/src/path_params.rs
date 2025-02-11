use std::sync::Arc;

use matchit::Params;

use crate::response_error::{InvalidUtf8InPathParams, InvalidUtf8InPathParamsSnafu};

#[derive(Debug, Clone)]
pub(crate) enum PathParams {
    Ok(Box<[(Arc<str>, Arc<str>)]>),
    Err(InvalidUtf8InPathParams),
}

impl PathParams {
    pub(crate) fn new(params: Params<'_, '_>) -> Self {
        let mut ok_params = Vec::with_capacity(params.len());
        let mut err_params = Vec::new();

        for (k, v) in params.iter() {
            let key = Arc::<str>::from(k);

            match percent_encoding::percent_decode(v.as_bytes()).decode_utf8() {
                Ok(o) => {
                    let value = Arc::<str>::from(o);
                    ok_params.push((key, value));
                }
                Err(_) => {
                    err_params.push(key);
                }
            }
        }

        if err_params.is_empty() {
            Self::Ok(ok_params.into())
        } else {
            Self::Err(InvalidUtf8InPathParamsSnafu { keys: err_params }.build())
        }
    }
}
