use std::net::{IpAddr, Ipv4Addr};

use predawn_core::request::DEFAULT_REQUEST_BODY_LIMIT;
use rudi::Singleton;
use serde::{Deserialize, Serialize};

use super::{Config, ConfigPrefix};
use crate::{normalized_path::NormalizedPath, path_util};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    #[serde(default = "default_ip")]
    pub ip: IpAddr,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_root_path")]
    root_path: String,
    #[serde(default = "default_non_application_root_path")]
    non_application_root_path: String,
    #[serde(default = "default_request_body_limit")]
    pub request_body_limit: usize,
}

#[Singleton]
impl ServerConfig {
    #[di]
    fn new(#[di(ref)] config: &Config) -> Self {
        config.get().unwrap_or_default()
    }

    #[track_caller]
    pub fn root_path(&self) -> &str {
        let root_path = &self.root_path;
        path_util::validate_path(root_path);
        root_path
    }

    #[track_caller]
    pub fn normalized_root_path(&self) -> NormalizedPath {
        NormalizedPath::new(self.root_path())
    }

    #[track_caller]
    pub fn non_application_root_path(&self) -> &str {
        let non_application_root_path = &self.non_application_root_path;
        path_util::validate_path(non_application_root_path);
        non_application_root_path
    }

    #[track_caller]
    pub fn normalized_non_application_root_path(&self) -> NormalizedPath {
        NormalizedPath::new(self.non_application_root_path())
    }

    #[track_caller]
    pub fn full_non_application_root_path(&self) -> NormalizedPath {
        let normalized_root_path = self.normalized_root_path();
        let normalized_non_application_root_path = self.normalized_non_application_root_path();

        normalized_root_path.join(normalized_non_application_root_path)
    }
}

fn default_ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
}

fn default_port() -> u16 {
    9612
}

fn default_root_path() -> String {
    "/".into()
}

fn default_non_application_root_path() -> String {
    "/p".into()
}

fn default_request_body_limit() -> usize {
    DEFAULT_REQUEST_BODY_LIMIT
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ip: default_ip(),
            port: default_port(),
            root_path: default_root_path(),
            non_application_root_path: default_non_application_root_path(),
            request_body_limit: default_request_body_limit(),
        }
    }
}

impl ConfigPrefix for ServerConfig {
    const PREFIX: &'static str = "server";
}
