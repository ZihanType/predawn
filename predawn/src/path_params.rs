use std::sync::Arc;

use matchit::Params;

#[derive(Clone, Debug)]
pub(crate) struct PathParams(pub(crate) Vec<(Arc<str>, Arc<str>)>);

impl PathParams {
    pub(crate) fn new(params: Params) -> Self {
        Self(
            params
                .iter()
                .map(|(k, v)| {
                    let key = Arc::<str>::from(k);
                    let value = Arc::<str>::from(v);

                    (key, value)
                })
                .collect(),
        )
    }
}
