use std::fmt;

use rudi::Singleton;
use serde::{Deserialize, Serialize};

use super::{Config, ConfigPrefix};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggerConfig {
    #[serde(default)]
    pub level: LogLevel,
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

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogLevel {
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

impl LogLevel {
    pub fn as_tracing_level(&self) -> Option<tracing::Level> {
        match self {
            LogLevel::Trace => Some(tracing::Level::TRACE),
            LogLevel::Debug => Some(tracing::Level::DEBUG),
            LogLevel::Info => Some(tracing::Level::INFO),
            LogLevel::Warn => Some(tracing::Level::WARN),
            LogLevel::Error => Some(tracing::Level::ERROR),
            LogLevel::Off => None,
        }
    }

    pub fn as_tracing_level_filter(&self) -> tracing::level_filters::LevelFilter {
        match self {
            LogLevel::Trace => tracing::level_filters::LevelFilter::TRACE,
            LogLevel::Debug => tracing::level_filters::LevelFilter::DEBUG,
            LogLevel::Info => tracing::level_filters::LevelFilter::INFO,
            LogLevel::Warn => tracing::level_filters::LevelFilter::WARN,
            LogLevel::Error => tracing::level_filters::LevelFilter::ERROR,
            LogLevel::Off => tracing::level_filters::LevelFilter::OFF,
        }
    }

    pub fn as_log_level(&self) -> Option<log::Level> {
        match self {
            LogLevel::Trace => Some(log::Level::Trace),
            LogLevel::Debug => Some(log::Level::Debug),
            LogLevel::Info => Some(log::Level::Info),
            LogLevel::Warn => Some(log::Level::Warn),
            LogLevel::Error => Some(log::Level::Error),
            LogLevel::Off => None,
        }
    }

    pub fn as_log_level_filter(&self) -> log::LevelFilter {
        match self {
            LogLevel::Trace => log::LevelFilter::Trace,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Off => log::LevelFilter::Off,
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "Trace"),
            LogLevel::Debug => write!(f, "Debug"),
            LogLevel::Info => write!(f, "Info"),
            LogLevel::Warn => write!(f, "Warn"),
            LogLevel::Error => write!(f, "Error"),
            LogLevel::Off => write!(f, "Off"),
        }
    }
}
