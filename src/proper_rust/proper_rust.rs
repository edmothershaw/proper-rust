use futures::join;
use warp::{Filter, Reply};

use crate::proper_rust::monitoring;
use crate::proper_rust::flow_logger::init_logging;
use crate::proper_rust::settings::{Settings, load_config};
use deadpool_postgres::Pool;
use crate::proper_rust::database::create_pool;

pub async fn start_server<F>(filter: F)
    where
        F: Filter + Clone + Send + Sync + 'static,
        F::Extract: Reply,
{
    let prometheus = warp::serve(
        warp::get()
            .and(warp::path("metrics"))
            .and_then(prometheus_metrics)
    ).run(([127, 0, 0, 1], 8080));

    let app = warp::serve(filter)
        .run(([127, 0, 0, 1], 3030));

    join!(prometheus, app);
}

pub fn setup() -> (Settings, Option<Pool>) {
    init_logging("log4rs.yml");

    let config = load_config();

    let pool_opt = if config.database.enabled {
        let pool = create_pool(&config);
        Some(pool)
    } else {
        None
    };

    (config, pool_opt)
}

async fn prometheus_metrics() -> Result<impl warp::Reply, warp::Rejection> {
    Ok(format!("{}", monitoring::metrics()))
}
