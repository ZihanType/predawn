use rudi::Singleton;
use serde::{Deserialize, Serialize};

use super::{Config, ConfigPrefix};
use crate::normalized_path::NormalizedPath;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenAPIConfig {
    #[serde(default = "default_json_path")]
    pub json_path: NormalizedPath,
    #[serde(default = "default_swagger_ui_path")]
    pub swagger_ui_path: NormalizedPath,
    #[serde(default = "default_rapidoc_path")]
    pub rapidoc_path: NormalizedPath,
}

#[Singleton]
impl OpenAPIConfig {
    #[di]
    fn new(#[di(ref)] config: &Config) -> Self {
        config.get().unwrap_or_default()
    }
}

fn default_json_path() -> NormalizedPath {
    "/openapi.json".into()
}

fn default_swagger_ui_path() -> NormalizedPath {
    "/swagger-ui".into()
}

fn default_rapidoc_path() -> NormalizedPath {
    "/rapidoc".into()
}

impl Default for OpenAPIConfig {
    fn default() -> Self {
        Self {
            json_path: default_json_path(),
            swagger_ui_path: default_swagger_ui_path(),
            rapidoc_path: default_rapidoc_path(),
        }
    }
}

impl ConfigPrefix for OpenAPIConfig {
    const PREFIX: &'static str = "openapi";
}
