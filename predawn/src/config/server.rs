use std::net::{IpAddr, Ipv4Addr};

use predawn_core::request::DEFAULT_BODY_LIMIT;
use rudi::Singleton;
use serde::{Deserialize, Serialize};

use super::{Config, ConfigPrefix};
use crate::normalized_path::NormalizedPath;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    #[serde(default = "default_ip")]
    pub ip: IpAddr,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_root_path")]
    pub root_path: NormalizedPath,
    #[serde(default = "default_non_application_root_path")]
    pub non_application_root_path: NormalizedPath,
    #[serde(default = "default_request_body_limit")]
    pub request_body_limit: usize,
}

#[Singleton(eager_create)]
impl ServerConfig {
    #[di]
    pub fn new(#[di(ref)] config: &Config) -> Self {
        config.get().expect("failed to load `ServerConfig`")
    }

    pub fn full_non_application_root_path(self) -> NormalizedPath {
        self.root_path.join(self.non_application_root_path)
    }
}

const fn default_ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
}

const fn default_port() -> u16 {
    9612
}

fn default_root_path() -> NormalizedPath {
    "/".into()
}

fn default_non_application_root_path() -> NormalizedPath {
    "/p".into()
}

const fn default_request_body_limit() -> usize {
    DEFAULT_BODY_LIMIT
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
