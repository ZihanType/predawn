use std::{fmt, str::FromStr};

use rudi::Singleton;
use serde::{Deserialize, Deserializer, Serialize};

use super::{Config, ConfigPrefix};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggerConfig {
    #[serde(default)]
    pub level: Level,
}

#[Singleton]
impl LoggerConfig {
    #[di]
    pub fn new(#[di(ref)] config: &Config) -> Self {
        config.get().expect("failed to load `LoggerConfig`")
    }
}

impl ConfigPrefix for LoggerConfig {
    const PREFIX: &'static str = "logger";
}

#[derive(Debug, Default, Copy, Clone, Serialize, PartialEq, Eq)]
pub enum Level {
    /// The "trace" level.
    Trace,
    /// The "debug" level.
    Debug,
    /// The "info" level.
    #[default]
    Info,
    /// The "warn" level.
    Warn,
    /// The "error" level.
    Error,
    /// Off level.
    Off,
}

impl<'de> Deserialize<'de> for Level {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const VARIANTS: [&str; 6] = ["trace", "debug", "info", "warn", "error", "off"];

        let s = String::deserialize(deserializer)?;
        s.parse()
            .map_err(|_| <D::Error as serde::de::Error>::unknown_variant(&s, &VARIANTS))
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct ParseLevelError;

impl FromStr for Level {
    type Err = ParseLevelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            s if s.eq_ignore_ascii_case("trace") => Ok(Level::Trace),
            s if s.eq_ignore_ascii_case("debug") => Ok(Level::Debug),
            s if s.eq_ignore_ascii_case("info") => Ok(Level::Info),
            s if s.eq_ignore_ascii_case("warn") => Ok(Level::Warn),
            s if s.eq_ignore_ascii_case("error") => Ok(Level::Error),
            s if s.eq_ignore_ascii_case("off") => Ok(Level::Off),
            _ => Err(ParseLevelError),
        }
    }
}

impl Level {
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Trace => "Trace",
            Level::Debug => "Debug",
            Level::Info => "Info",
            Level::Warn => "Warn",
            Level::Error => "Error",
            Level::Off => "Off",
        }
    }

    pub fn as_tracing_level(&self) -> Option<tracing::Level> {
        match self {
            Level::Trace => Some(tracing::Level::TRACE),
            Level::Debug => Some(tracing::Level::DEBUG),
            Level::Info => Some(tracing::Level::INFO),
            Level::Warn => Some(tracing::Level::WARN),
            Level::Error => Some(tracing::Level::ERROR),
            Level::Off => None,
        }
    }

    pub fn as_tracing_level_filter(&self) -> tracing::level_filters::LevelFilter {
        match self {
            Level::Trace => tracing::level_filters::LevelFilter::TRACE,
            Level::Debug => tracing::level_filters::LevelFilter::DEBUG,
            Level::Info => tracing::level_filters::LevelFilter::INFO,
            Level::Warn => tracing::level_filters::LevelFilter::WARN,
            Level::Error => tracing::level_filters::LevelFilter::ERROR,
            Level::Off => tracing::level_filters::LevelFilter::OFF,
        }
    }

    pub fn as_log_level(&self) -> Option<log::Level> {
        match self {
            Level::Trace => Some(log::Level::Trace),
            Level::Debug => Some(log::Level::Debug),
            Level::Info => Some(log::Level::Info),
            Level::Warn => Some(log::Level::Warn),
            Level::Error => Some(log::Level::Error),
            Level::Off => None,
        }
    }

    pub fn as_log_level_filter(&self) -> log::LevelFilter {
        match self {
            Level::Trace => log::LevelFilter::Trace,
            Level::Debug => log::LevelFilter::Debug,
            Level::Info => log::LevelFilter::Info,
            Level::Warn => log::LevelFilter::Warn,
            Level::Error => log::LevelFilter::Error,
            Level::Off => log::LevelFilter::Off,
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.as_str())
    }
}
