use std::{env, error::Error, fmt, num::ParseIntError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub supabase_url: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let host = read_env_with_default("API_HOST", "127.0.0.1")?;
        let port = match env::var("API_PORT") {
            Ok(value) => value
                .parse()
                .map_err(|source| ConfigError::InvalidPort { value, source })?,
            Err(env::VarError::NotPresent) => 4000,
            Err(env::VarError::NotUnicode(_)) => {
                return Err(ConfigError::InvalidUnicode("API_PORT"))
            }
        };
        let database_url = read_required_env("DATABASE_URL")?;
        let supabase_url = read_required_env("SUPABASE_URL")?;
        let supabase_url = supabase_url.trim_end_matches('/').to_owned();

        Ok(Self {
            host,
            port,
            database_url,
            supabase_url,
        })
    }

    pub fn supabase_issuer(&self) -> String {
        format!("{}/auth/v1", self.supabase_url)
    }

    pub fn supabase_jwks_url(&self) -> String {
        format!("{}/auth/v1/.well-known/jwks.json", self.supabase_url)
    }
}

fn read_env_with_default(name: &'static str, default: &str) -> Result<String, ConfigError> {
    match env::var(name) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok(default.to_owned()),
        Err(env::VarError::NotUnicode(_)) => Err(ConfigError::InvalidUnicode(name)),
    }
}

fn read_required_env(name: &'static str) -> Result<String, ConfigError> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        Ok(_) | Err(env::VarError::NotPresent) => Err(ConfigError::MissingRequired(name)),
        Err(env::VarError::NotUnicode(_)) => Err(ConfigError::InvalidUnicode(name)),
    }
}

#[derive(Debug)]
pub enum ConfigError {
    MissingRequired(&'static str),
    InvalidUnicode(&'static str),
    InvalidPort {
        value: String,
        source: ParseIntError,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequired(name) => {
                write!(formatter, "missing required environment variable {name}")
            }
            Self::InvalidUnicode(name) => write!(
                formatter,
                "environment variable {name} must be valid unicode"
            ),
            Self::InvalidPort { value, .. } => {
                write!(formatter, "API_PORT must be a valid u16, got {value}")
            }
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPort { source, .. } => Some(source),
            Self::MissingRequired(_) | Self::InvalidUnicode(_) => None,
        }
    }
}
