use std::collections::BTreeMap;

use predawn::config::{Config, ConfigPrefix};
use rudi::Singleton;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataSourcesConfig {
    pub default: Option<Url>,
    #[serde(flatten)]
    pub data_sources: BTreeMap<String, Url>,
}

#[Singleton]
impl From<&Config> for DataSourcesConfig {
    #[di]
    #[track_caller]
    fn from(#[di(ref)] config: &Config) -> Self {
        config.get().expect("failed to load `DataSourcesConfig`")
    }
}

impl ConfigPrefix for DataSourcesConfig {
    const PREFIX: &'static str = "data_sources";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Url {
    Simple(String),
    Detailed(UrlDetail),
}

impl From<Url> for String {
    fn from(url: Url) -> Self {
        match url {
            Url::Simple(s) => s,
            Url::Detailed(d) => d.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlDetail {
    pub url: url::Url,
    pub username: String,
    pub password: String,
}

impl From<UrlDetail> for String {
    fn from(url: UrlDetail) -> Self {
        let UrlDetail {
            mut url,
            username,
            password,
        } = url;

        let _ = url.set_username(&username);
        let _ = url.set_password(Some(&password));

        url.into()
    }
}
