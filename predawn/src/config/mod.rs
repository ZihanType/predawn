pub mod logger;
pub mod openapi;
pub mod server;

use std::{
    env,
    ops::Deref,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use config::{ConfigError, File, ValueKind};
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
        static DEFAULT_FOLDER: LazyLock<PathBuf> = LazyLock::new(|| {
            let mut dir_path = match env::var("CARGO_MANIFEST_DIR") {
                Ok(dir) => PathBuf::from(dir),
                Err(_) => {
                    let binary_file_path =
                        env::args().next().expect("failed to get binary file path");

                    Path::new(&binary_file_path)
                        .parent()
                        .expect("failed to get parent directory of binary file")
                        .canonicalize()
                        .expect("failed to get canonical, absolute form of the path to the parent directory of binary file")
                }
            };

            dir_path.push("config");
            dir_path
        });

        Self::from_folder(env, DEFAULT_FOLDER.as_path())
    }

    pub fn from_folder(env: &Environment, path: &Path) -> Result<Self, ConfigError> {
        let app_cfg = path.join("app.toml");
        let env_cfg = path.join(format!("app-{}.toml", env));
        let env = config::Environment::default().separator("_");

        let mut builder = config::Config::builder();

        for cfg in [app_cfg, env_cfg].into_iter() {
            tracing::info!("try to load config `{}`", cfg.display());

            if cfg.exists() {
                builder = builder.add_source(File::from(cfg))
            } else {
                tracing::info!("not found config `{}`", cfg.display());
            }
        }

        let config = builder.add_source(env).build()?;

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
