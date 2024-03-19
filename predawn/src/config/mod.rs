pub mod logger;
pub mod openapi;
pub mod server;

use std::{
    env,
    ops::Deref,
    path::{Path, PathBuf},
};

use config::{ConfigError, File, ValueKind};
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::environment::Environment;

#[derive(Clone)]
pub struct Config {
    inner: config::Config,
}

impl Config {
    pub fn new(config: config::Config) -> Self {
        Self { inner: config }
    }

    pub fn load(env: &Environment) -> Result<Self, ConfigError> {
        static DEFAULT_FOLDER: Lazy<PathBuf> = Lazy::new(|| {
            let mut parent = match env::var("CARGO_MANIFEST_DIR") {
                Ok(dir) => PathBuf::from(dir),
                Err(_) => {
                    let binary_file_path = env::args()
                        .next()
                        .expect("unreachable: there must be a binary file path");

                    Path::new(&binary_file_path)
                        .parent()
                        .expect("unreachable: there must be a parent directory")
                        .canonicalize()
                        .expect("unreachable: the parent directory must be valid")
                }
            };

            parent.push("config");
            parent
        });

        Self::from_folder(env, DEFAULT_FOLDER.as_path())
    }

    pub fn from_folder(env: &Environment, path: &Path) -> Result<Self, ConfigError> {
        let app_cfg = path.join("app.toml");
        let env_cfg = path.join(format!("app-{}.toml", env));
        let env = config::Environment::default().separator("_");

        tracing::info!("trying to load configuration from `{}`", app_cfg.display());
        if !app_cfg.exists() {
            tracing::info!("`{}` does not exist", app_cfg.display());
        }

        tracing::info!("trying to load configuration from `{}`", env_cfg.display());
        if !env_cfg.exists() {
            tracing::info!("`{}` does not exist", env_cfg.display());
        }

        let config = config::Config::builder()
            .add_source(File::from(app_cfg).required(false))
            .add_source(File::from(env_cfg).required(false))
            .add_source(env)
            .build()?;

        Ok(Self { inner: config })
    }

    pub fn is_debug(&self) -> bool {
        self.inner.get_bool("debug").unwrap_or_default()
    }

    pub fn get<'de, T>(&self) -> Result<T, ConfigError>
    where
        T: ConfigPrefix + Deserialize<'de>,
    {
        match self.inner.get::<T>(T::PREFIX) {
            Ok(o) => Ok(o),
            Err(e) => {
                let ConfigError::NotFound(_) = &e else {
                    return Err(e);
                };

                let v = config::Value::new(None, ValueKind::Table(Default::default()));

                match T::deserialize(v) {
                    Ok(o) => Ok(o),
                    Err(_) => Err(e),
                }
            }
        }
    }
}

impl Deref for Config {
    type Target = config::Config;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub trait ConfigPrefix {
    const PREFIX: &'static str;
}
