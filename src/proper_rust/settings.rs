use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

use crate::proper_rust::flow_logger::{FlowContext, FlowLogger};

#[derive(Debug, Deserialize)]
pub struct Database {
    pub enabled: bool,
    pub url: String,
    pub username: String,
    pub password: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub database: Database,
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
    let logger = FlowLogger::new("app::backend::db");

    let fc = FlowContext { flow_id: "config-load".to_string() };

    logger.info(&fc, "Loading configuration");

    let conf_result = Settings::new();

    match conf_result {
        Ok(res) => {
            logger.info(&fc, res.database.url.as_str());
            res
        }
        Err(err) => {
            logger.error(&fc, err.to_string().as_str());
            panic!("failed to config");
        }
    }
}
