use rudi::Singleton;
use serde::{Deserialize, Serialize};

use super::{Config, ConfigPrefix};
use crate::{normalized_path::NormalizedPath, path_util};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenAPIConfig {
    #[serde(default = "default_json_path")]
    json_path: String,
    #[serde(default = "default_swagger_ui_path")]
    swagger_ui_path: String,
    #[serde(default = "default_rapidoc_path")]
    rapidoc_path: String,
}

#[Singleton]
impl OpenAPIConfig {
    #[di]
    fn new(#[di(ref)] config: &Config) -> Self {
        config.get().unwrap_or_default()
    }

    #[track_caller]
    pub fn json_path(&self) -> &str {
        let json_path = &self.json_path;
        path_util::validate_path(json_path);
        json_path
    }

    #[track_caller]
    pub fn normalized_json_path(&self) -> NormalizedPath {
        NormalizedPath::new(self.json_path())
    }

    #[track_caller]
    pub fn swagger_ui_path(&self) -> &str {
        let swagger_ui_path = &self.swagger_ui_path;
        path_util::validate_path(swagger_ui_path);
        swagger_ui_path
    }

    #[track_caller]
    pub fn normalized_swagger_ui_path(&self) -> NormalizedPath {
        NormalizedPath::new(self.swagger_ui_path())
    }

    #[track_caller]
    pub fn rapidoc_path(&self) -> &str {
        let rapidoc_path = &self.rapidoc_path;
        path_util::validate_path(rapidoc_path);
        rapidoc_path
    }

    #[track_caller]
    pub fn normalized_rapidoc_path(&self) -> NormalizedPath {
        NormalizedPath::new(self.rapidoc_path())
    }
}

fn default_json_path() -> String {
    "/openapi.json".into()
}

fn default_swagger_ui_path() -> String {
    "/swagger-ui".into()
}

fn default_rapidoc_path() -> String {
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
