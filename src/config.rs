use secrecy::{ExposeSecret, SecretString};

#[derive(serde::Deserialize)]
pub struct AppConfig {
    pub database: DbConfig,
    pub app_port: u16,
}

#[derive(serde::Deserialize, Clone)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub db_name: String,
    pub username: String,
    pub password: SecretString,
}

impl DbConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.db_name
        )
    }
}

pub fn get_config() -> Result<AppConfig, config::ConfigError> {
    let settings = config::Config::builder()
        .add_source(config::File::new("config.yaml", config::FileFormat::Yaml))
        .build()?;
    settings.try_deserialize::<AppConfig>()
}
