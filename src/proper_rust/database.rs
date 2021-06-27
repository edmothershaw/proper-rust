use std::collections::HashMap;
use std::sync::Arc;

use deadpool_postgres::{Config, Manager, ManagerConfig, Pool, RecyclingMethod};
use futures::join;
use log::{error, info, Record};
use log4rs;
use log_mdc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;
use url::{ParseError, Url};
use warp::{Filter, http, Reply};

use crate::proper_rust::settings::Settings;

pub fn create_pool(settings: Settings) -> Pool {
    let mut cfg = Config::new();

    let url = Url::parse(settings.database.url.as_str()).unwrap();
    let url_path: Vec<String> = vec!(url.path_segments().unwrap().collect());
    let host = url.host_str().unwrap().to_string();
    let dbname = url_path.first().unwrap().to_string();

    cfg.dbname = Some(dbname);
    cfg.host = Some(host);
    cfg.user = Some(settings.database.username);
    cfg.port = Some(settings.database.port);
    cfg.password = Some(settings.database.password);
    cfg.manager = Some(ManagerConfig { recycling_method: RecyclingMethod::Fast });

    cfg.create_pool(NoTls).unwrap()
}