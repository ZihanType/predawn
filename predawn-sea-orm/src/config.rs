use std::{collections::HashMap, time::Duration};

use predawn::config::{logger::Level, Config, ConfigPrefix};
use rudi::Singleton;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataSourcesConfig {
    #[serde(flatten)]
    pub data_sources: HashMap<String, ConnectOptions>,
}

#[Singleton]
impl DataSourcesConfig {
    #[di]
    pub fn new(#[di(ref)] config: &Config) -> Self {
        config.get().expect("failed to load `DataSourcesConfig`")
    }
}

impl ConfigPrefix for DataSourcesConfig {
    const PREFIX: &'static str = "data_sources";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlowStatementsLoggingSettings {
    #[serde(default)]
    pub level: Level,
    #[serde(deserialize_with = "duration_str::deserialize_duration")]
    #[serde(default)]
    pub threshold: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectOptions {
    pub url: Url,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,

    #[serde(default)]
    pub max_connections: Option<u32>,
    #[serde(default)]
    pub min_connections: Option<u32>,

    #[serde(deserialize_with = "duration_str::deserialize_option_duration")]
    #[serde(default)]
    pub connect_timeout: Option<Duration>,
    #[serde(deserialize_with = "duration_str::deserialize_option_duration")]
    #[serde(default)]
    pub idle_timeout: Option<Duration>,
    #[serde(deserialize_with = "duration_str::deserialize_option_duration")]
    #[serde(default)]
    pub acquire_timeout: Option<Duration>,
    #[serde(deserialize_with = "duration_str::deserialize_option_duration")]
    #[serde(default)]
    pub max_lifetime: Option<Duration>,

    #[serde(default)]
    pub sqlx_logging: Option<bool>,
    #[serde(default)]
    pub sqlx_logging_level: Option<Level>,
    #[serde(default)]
    pub sqlx_slow_statements_logging_settings: Option<SlowStatementsLoggingSettings>,

    #[serde(default)]
    pub sqlcipher_key: Option<String>,
    #[serde(default)]
    pub schema_search_path: Option<String>,
    #[serde(default)]
    pub test_before_acquire: Option<bool>,
    #[serde(default)]
    pub connect_lazy: Option<bool>,
}

impl From<ConnectOptions> for sea_orm::ConnectOptions {
    fn from(options: ConnectOptions) -> Self {
        let ConnectOptions {
            mut url,
            username,
            password,
            max_connections,
            min_connections,
            connect_timeout,
            idle_timeout,
            acquire_timeout,
            max_lifetime,
            sqlx_logging,
            sqlx_logging_level,
            sqlx_slow_statements_logging_settings,
            sqlcipher_key,
            schema_search_path,
            test_before_acquire,
            connect_lazy,
        } = options;

        if let Some(username) = username {
            url.set_username(&username)
                .unwrap_or_else(|_| panic!("failed to set username: {username} to url: {url}"));
        }

        if let Some(password) = password {
            url.set_password(Some(&password))
                .unwrap_or_else(|_| panic!("failed to set password: {password} to url: {url}"));
        }

        let mut options = sea_orm::ConnectOptions::new(url.to_string());

        if let Some(max_connections) = max_connections {
            options.max_connections(max_connections);
        }

        if let Some(min_connections) = min_connections {
            options.min_connections(min_connections);
        }

        if let Some(connect_timeout) = connect_timeout {
            options.connect_timeout(connect_timeout);
        }

        if let Some(idle_timeout) = idle_timeout {
            options.idle_timeout(idle_timeout);
        }

        if let Some(acquire_timeout) = acquire_timeout {
            options.acquire_timeout(acquire_timeout);
        }

        if let Some(max_lifetime) = max_lifetime {
            options.max_lifetime(max_lifetime);
        }

        if let Some(sqlx_logging) = sqlx_logging {
            options.sqlx_logging(sqlx_logging);
        }

        if let Some(sqlx_logging_level) = sqlx_logging_level {
            options.sqlx_logging_level(sqlx_logging_level.as_log_level_filter());
        }

        if let Some(SlowStatementsLoggingSettings { level, threshold }) =
            sqlx_slow_statements_logging_settings
        {
            options.sqlx_slow_statements_logging_settings(level.as_log_level_filter(), threshold);
        }

        if let Some(sqlcipher_key) = sqlcipher_key {
            options.sqlcipher_key(sqlcipher_key);
        }

        if let Some(schema_search_path) = schema_search_path {
            options.set_schema_search_path(schema_search_path);
        }

        if let Some(test_before_acquire) = test_before_acquire {
            options.test_before_acquire(test_before_acquire);
        }

        if let Some(connect_lazy) = connect_lazy {
            options.connect_lazy(connect_lazy);
        }

        options
    }
}
