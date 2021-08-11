use std::collections::HashMap;
use std::sync::Arc;

use deadpool_postgres::Pool;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use warp::{Filter, http};

use proper_rust::flow_logger::{FlowContext, FlowLogger};
use proper_rust::monitoring::{ErrorTagger, timed};

use crate::api::*;

mod api;
mod proper_rust;

type Items = HashMap<String, i32>;

lazy_static! {
    static ref LOG: FlowLogger = FlowLogger::new("app::backend");
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Item {
    name: String,
    quantity: i32,
}

#[derive(Clone)]
struct Store {
    grocery_list: Arc<RwLock<Items>>,
}

impl Store {
    fn new() -> Self {
        Store {
            grocery_list: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}


async fn add_grocery_list_item(
    item: Item,
    store: Store,
    _fc: FlowContext,
) -> Result<impl warp::Reply, warp::Rejection> {
    store.grocery_list.write().insert(item.name, item.quantity);

    Ok(warp::reply::with_status(
        "Added items to the grocery list",
        http::StatusCode::CREATED,
    ))
}

fn json_body() -> impl Filter<Extract=(Item, ), Error=warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

async fn get_grocery_list(
    store: Store,
    fc: FlowContext,
) -> Result<impl warp::Reply, warp::Rejection> {
    timed("get_grocery_list", || {
        async {
            let mut result = HashMap::new();
            let r = store.grocery_list.read();

            for (key, value) in r.iter() {
                result.insert(key, value);
            }

            LOG.info(&fc, "Fetched grocery list");

            Ok(warp::reply::json(
                &result
            ))
        }
    }).await
}

impl ErrorTagger for warp::Rejection {
    fn error_tag(&self) -> String {
        match self {
            _ => "rejection".to_string()
        }
    }
}


async fn chuck(pool: Pool, chuck_api: impl ChuckApiService, fc: FlowContext) -> Result<impl warp::Reply, warp::Rejection> {
    timed("chuck", || {
        async {
            LOG.info(&fc, "making api call");

            let res = chuck_api.make_call().await;

            let res2 = match res {
                Ok(a) => Ok(a),
                Err(_) => Err(warp::reject()),
            }?;

            write_chuck(pool, &res2).await;

            Ok(warp::reply::json(
                &res2
            ))
        }
    }).await
}


async fn db_run(pool: &Pool) {
    let fc = FlowContext::new("db-flow");
    for i in 1..10 {
        let client = pool.get().await.unwrap();
        let stmt = client.prepare_cached("SELECT 1 + $1").await.unwrap();
        let rows = client.query(&stmt, &[&i]).await.unwrap();
        let value: i32 = rows[0].get(0);
        LOG.info(&fc, value.to_string().as_str());
        assert_eq!(value, i + 1);
    }
}

async fn write_chuck(pool: Pool, chuck: &Chuck) {
    let client = pool.get().await.unwrap();
    let stmt = client.prepare_cached("INSERT INTO rust_test.chuck(value) VALUES ($1)").await.unwrap();
    let _rows = client.query(&stmt, &[&chuck.value]).await.unwrap();
}

fn ping1() -> impl Filter<Extract=(String,), Error=warp::Rejection> + Clone {
    warp::get()
        .and(warp::path("ping"))
        .map(|| format!("{{\"status\":\"OK\"}}"))
}

fn ping2() -> impl Filter<Extract=(String,), Error=warp::Rejection> + Clone {
    warp::get()
        .and(warp::path("ping"))
        .map(|| format!("{{\"status\":\"OK\"}}"))
}

#[tokio::main]
async fn main() {
    let (_config, pool_opt) = proper_rust::setup();
    let pool = pool_opt.unwrap();

    db_run(&pool).await;

    let store = Store::new();
    let store_filter = warp::any().map(move || {
        store.clone()
    });

    let pool_filter = warp::any().map(move || {
        pool.clone()
    });

    let chuck_api_service_filter = warp::any().map(move || {
        ChuckApiServiceImpl::new(ChuckConfig { url: "https://api.chucknorris.io/jokes/random".to_string() })
    });

    let add_items = warp::post()
        .and(warp::path("v1"))
        .and(warp::path("groceries"))
        .and(warp::path::end())
        .and(json_body())
        .and(store_filter.clone())
        .and(FlowContext::extract_flow_context())
        .and_then(add_grocery_list_item);

    let get_items = warp::get()
        .and(warp::path("v1"))
        .and(warp::path("groceries"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and(FlowContext::extract_flow_context())
        .and_then(get_grocery_list);

    let chuck = warp::get()
        .and(warp::path("v1"))
        .and(warp::path("chuck"))
        .and(warp::path::end())
        .and(pool_filter.clone())
        .and(chuck_api_service_filter.clone())
        .and(FlowContext::extract_flow_context())
        .and_then(chuck);


    let routes =
        ping1().or(ping2())
            .or(add_items)
            .or(get_items)
            .or(chuck);

    proper_rust::start_server(_config, routes).await;
}


#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use reqwest::Error;
    use warp::hyper::body::HttpBody;
    use warp::Reply;

    use crate::proper_rust::database::create_pool;
    use crate::proper_rust::settings::Database;

    use super::*;

    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    struct MockChuckApiService;

    #[async_trait]
    impl ChuckApiService for MockChuckApiService {
        async fn make_call(&self) -> Result<Chuck, Error> {
            Ok(Chuck { value: "blah".to_string() })
        }
    }

    #[test]
    fn test_chuck() {
        let mock_chuck = MockChuckApiService;

        let pool = create_pool(&Database {
            enabled: false,
            url: "postgresql://localhost/create_drop".to_string(),
            username: "postgres".to_string(),
            password: "asdf123".to_string(),
            port: 5432,
        });

        let fc = FlowContext {
            flow_id: "my-flow".to_string()
        };
        let res = aw!(chuck(pool, mock_chuck, fc));

        match res {
            Ok(r) => {
                let resp = warp_reply(r);
                assert_eq!(resp, "{\"value\":\"blah\"}")
            },
            Err(_) => assert_eq!("should not reject", ""),
        }
    }

    fn warp_reply(r: impl Reply) -> String {
        let response = r.into_response();
        let body = aw!(response.into_body().data());
        let b = body.unwrap().unwrap();
        String::from_utf8_lossy(&b.to_vec()).to_string()
    }

}
