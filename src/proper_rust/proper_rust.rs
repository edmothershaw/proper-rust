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
use warp::{Filter, http, Reply};

use crate::proper_rust::monitoring;

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

async fn prometheus_metrics() -> Result<impl warp::Reply, warp::Rejection> {
    Ok(format!("{}", monitoring::metrics()))
}
