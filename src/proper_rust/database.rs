use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::NoTls;
use url::Url;

use crate::proper_rust::settings::Settings;

pub fn create_pool(settings: &Settings) -> Pool {
    let mut cfg = Config::new();

    let url = Url::parse(settings.database.url.as_str()).unwrap();
    let url_path: Vec<String> = vec!(url.path_segments().unwrap().collect());
    let host = url.host_str().unwrap().to_string();
    let dbname = url_path.first().unwrap().to_string();

    cfg.dbname = Some(dbname);
    cfg.host = Some(host);
    cfg.port = Some(settings.database.port);
    cfg.user = Some(settings.database.username.to_string());
    cfg.password = Some(settings.database.password.to_string());
    cfg.manager = Some(ManagerConfig { recycling_method: RecyclingMethod::Fast });

    cfg.create_pool(NoTls).unwrap()
}
