use std::convert::Infallible;

use deadpool_postgres::Pool;
use futures::join;
use warp::{Filter, Rejection, Reply, Error};
use warp::filters::path::Exact;

use crate::proper_rust::database::create_pool;
use crate::proper_rust::flow_logger::init_logging;
use crate::proper_rust::monitoring;
use crate::proper_rust::settings::{load_config, Settings};

pub async fn start_server<F>(config: Settings, filter: F, f2: F)
    where
        F: Filter + Clone + Send + Sync + 'static,
        F::Extract: Reply
{
    let prometheus = warp::serve(
        warp::get()
            .and(warp::path("metrics"))
            .and_then(prometheus_metrics)
    ).run(([127, 0, 0, 1], 1234));

    let endpoints = filter.or(f2);

    let app = warp::serve(endpoints)
        .run(([127, 0, 0, 1], 3030));

    join!(prometheus, app);
}

fn ping1() -> impl Filter<Extract=(String,), Error=warp::Rejection> + Clone {
    warp::get()
        .and(warp::path("ping"))
        .map(|| format!("{{\"status\":\"OK\"}}"))
}

// fn rest_endpoints()
// {
//     let ping = warp::path!("ping")
//         .map(|| format!("{{\"status\":\"OK\"}}"));
//
//     let health = warp::path!("health")
//         .map(|| format!("{{}}"));
//
//     let version = warp::path!("version")
//         .map(|| format!("{{\"version\":\"{}\"}}", "blah"));
//
//     ping.or(health).or(version)
// }

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
