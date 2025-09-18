use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub app: AppConfig,
    pub db: DbConfig,
}

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    pub port: u16,
    pub host: String,
}

impl AppConfig {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub db_name: String,
    pub username: String,
    pub password: SecretString,
    pub require_ssl: bool,
}

impl DbConfig {
    pub fn connect_options(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .database(&self.db_name)
            .username(&self.username)
            .password(self.password.expose_secret())
            .ssl_mode(if self.require_ssl {
                PgSslMode::Require
            } else {
                PgSslMode::Prefer
            })
    }
}

pub fn get_config() -> Result<Config, anyhow::Error> {
    let settings = config::Config::builder()
        .add_source(config::File::new("config.yaml", config::FileFormat::Yaml))
        .add_source(
            config::Environment::with_prefix("CUSTOM")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;
    Ok(settings.try_deserialize::<Config>()?)
}
