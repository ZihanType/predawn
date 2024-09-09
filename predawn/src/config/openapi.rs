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
    #[serde(default = "default_scalar_path")]
    pub scalar_path: NormalizedPath,
    #[serde(default = "default_redoc_path")]
    pub redoc_path: NormalizedPath,
    #[serde(default = "default_openapi_explorer_path")]
    pub openapi_explorer_path: NormalizedPath,
}

#[Singleton]
impl OpenAPIConfig {
    #[di]
    pub fn new(#[di(ref)] config: &Config) -> Self {
        config.get().unwrap()
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

fn default_scalar_path() -> NormalizedPath {
    "/scalar".into()
}

fn default_redoc_path() -> NormalizedPath {
    "/redoc".into()
}

fn default_openapi_explorer_path() -> NormalizedPath {
    "/openapi-explorer".into()
}

impl Default for OpenAPIConfig {
    fn default() -> Self {
        Self {
            json_path: default_json_path(),
            swagger_ui_path: default_swagger_ui_path(),
            rapidoc_path: default_rapidoc_path(),
            scalar_path: default_scalar_path(),
            redoc_path: default_redoc_path(),
            openapi_explorer_path: default_openapi_explorer_path(),
        }
    }
}

impl ConfigPrefix for OpenAPIConfig {
    const PREFIX: &'static str = "openapi";
}
