use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::NoTls;
use url::Url;

use crate::proper_rust::settings::Database;

pub fn create_pool(database: &Database) -> Pool {
    let mut cfg = Config::new();

    let url = Url::parse(database.url.as_str()).unwrap();
    let url_path: Vec<String> = vec!(url.path_segments().unwrap().collect());
    let host = url.host_str().unwrap().to_string();
    let dbname = url_path.first().unwrap().to_string();

    cfg.dbname = Some(dbname);
    cfg.host = Some(host);
    cfg.port = Some(database.port);
    cfg.user = Some(database.username.to_string());
    cfg.password = Some(database.password.to_string());
    cfg.manager = Some(ManagerConfig { recycling_method: RecyclingMethod::Fast });

    cfg.create_pool(NoTls).unwrap()
}
