use std::{env, fmt};

use serde::{Deserialize, Serialize};

pub const PREDAWN_ENV: &str = "PREDAWN_ENV";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum Environment {
    #[serde(rename = "prod")]
    Prod,

    #[serde(rename = "dev")]
    Dev,

    #[serde(rename = "test")]
    Test,

    #[serde(untagged)]
    Custom(String),
}

impl Environment {
    pub fn resolve_from_env() -> Self {
        match env::var(PREDAWN_ENV) {
            Ok(e) => Self::from(e),
            Err(_) => {
                if cfg!(debug_assertions) {
                    Environment::Dev
                } else {
                    Environment::Prod
                }
            }
        }
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Environment::Prod => write!(f, "prod"),
            Environment::Dev => write!(f, "dev"),
            Environment::Test => write!(f, "test"),
            Environment::Custom(c) => c.fmt(f),
        }
    }
}

impl From<String> for Environment {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "prod" | "production" => Environment::Prod,
            "dev" | "development" => Environment::Dev,
            "test" => Environment::Test,
            _ => Environment::Custom(s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_serialize() {
        let env = Environment::Prod;
        let serialized = serde_json::to_string(&env).unwrap();
        assert_eq!(serialized, "\"prod\"");

        let env = Environment::Dev;
        let serialized = serde_json::to_string(&env).unwrap();
        assert_eq!(serialized, "\"dev\"");

        let env = Environment::Test;
        let serialized = serde_json::to_string(&env).unwrap();
        assert_eq!(serialized, "\"test\"");

        let env = Environment::Custom("foo".to_string());
        let serialized = serde_json::to_string(&env).unwrap();
        assert_eq!(serialized, "\"foo\"");
    }

    #[test]
    fn test_resolve_from_env() {
        let original = env::var(PREDAWN_ENV);

        env::remove_var(PREDAWN_ENV);
        assert_eq!(Environment::resolve_from_env(), Environment::Dev);

        env::set_var(PREDAWN_ENV, "foo");
        assert_eq!(
            Environment::resolve_from_env(),
            Environment::Custom("foo".to_string())
        );

        if let Ok(v) = original {
            env::set_var(PREDAWN_ENV, v);
        }
    }

    #[test]
    fn test_display() {
        assert_eq!("prod", Environment::Prod.to_string());
        assert_eq!("foo", Environment::Custom("foo".to_string()).to_string());
    }

    #[test]
    fn test_into() {
        let e: Environment = "PROD".to_string().into();
        assert_eq!(e, Environment::Prod);

        let e: Environment = "FOO".to_string().into();
        assert_eq!(e, Environment::Custom("FOO".to_string()));
    }
}
