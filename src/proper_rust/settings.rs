use config::{Config, ConfigError, Environment, File};
use log::error;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Database {
    pub enabled: bool,
    pub url: String,
    pub username: String,
    pub password: String,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoggingMeta {
    pub build_time: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub database: Database,
    pub log_file: Option<String>,
    pub service: LoggingMeta,
}


impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::default();
        s.merge(File::with_name("config/settings"))?;
        let environment = Environment::new();
        let res = Environment::separator(environment, "_");
        s.merge(res)?;
        s.try_into()
    }
}

pub fn load_config() -> Settings {
    let conf_result = Settings::new();
    match conf_result {
        Ok(res) => res,
        Err(err) => {
            error!("{}", err);
            panic!("failed to config")
        }
    }
}
