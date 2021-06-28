use futures::join;
use warp::{Filter, Reply};

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
