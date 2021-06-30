use deadpool_postgres::Pool;
use futures::join;
use warp::{Filter, Reply};

use crate::proper_rust::database::create_pool;
use crate::proper_rust::flow_logger::init_logging;
use crate::proper_rust::monitoring;
use crate::proper_rust::settings::{load_config, Settings};

pub async fn start_server<F>(filter: F)
    where
        F: Filter + Clone + Send + Sync + 'static,
        F::Extract: Reply,
{
    let prometheus = warp::serve(
        warp::get()
            .and(warp::path("metrics"))
            .and_then(prometheus_metrics)
    ).run(([127, 0, 0, 1], 1234));

    let app = warp::serve(filter)
        .run(([127, 0, 0, 1], 3030));

    join!(prometheus, app);
}

pub fn setup() -> (Settings, Option<Pool>) {
    let config: Settings = load_config();

    init_logging(&config);

    let pool_opt = if config.database.enabled {
        let pool = create_pool(&config.database);
        Some(pool)
    } else {
        None
    };

    (config, pool_opt)
}

async fn prometheus_metrics() -> Result<impl warp::Reply, warp::Rejection> {
    Ok(format!("{}", monitoring::metrics()))
}
