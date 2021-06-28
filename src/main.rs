use std::collections::HashMap;
use std::sync::Arc;

use deadpool_postgres::Pool;
use log::info;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use warp::{Filter, http};

use proper_rust::database::create_pool;
use proper_rust::flow_logger::init_logging;
use proper_rust::monitoring::timed;
use proper_rust::settings::load_config;

use crate::proper_rust::settings::{Settings};
use crate::api::Chuck;

mod api;
mod proper_rust;

type Items = HashMap<String, i32>;

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
    store: Store
) -> Result<impl warp::Reply, warp::Rejection> {
    timed("get_grocery_list", || {
        async {
            let mut result = HashMap::new();
            let r = store.grocery_list.read();

            for (key, value) in r.iter() {
                result.insert(key, value);
            }

            info!(target: "app::backend::db", "Fetched grocery list");

            Ok(warp::reply::json(
                &result
            ))
        }
    }).await
}


async fn chuck(pool: Pool) -> Result<impl warp::Reply, warp::Rejection> {
    timed("chuck", || {
        async {
            let res = api::make_call().await;

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
    for i in 1..10 {
        let client = pool.get().await.unwrap();
        let stmt = client.prepare_cached("SELECT 1 + $1").await.unwrap();
        let rows = client.query(&stmt, &[&i]).await.unwrap();
        let value: i32 = rows[0].get(0);
        info!(target: "app::backend::db", "{}", value);
        assert_eq!(value, i + 1);
    }
}

async fn write_chuck(pool: Pool, chuck: &Chuck) {
    let client = pool.get().await.unwrap();
    let stmt = client.prepare_cached("INSERT INTO rust_test.chuck(value) VALUES ($1)").await.unwrap();
    let _rows = client.query(&stmt, &[&chuck.value]).await.unwrap();
}

fn setup() -> (Settings, Option<Pool>) {
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

#[tokio::main]
async fn main() {
    let (_config, pool_opt) = setup();
    let pool = pool_opt.unwrap();

    db_run(&pool).await;

    let store = Store::new();
    let store_filter = warp::any().map(move || {
        store.clone()
    });

    let pool_filter = warp::any().map(move || {
        pool.clone()
    });

    let add_items = warp::post()
        .and(warp::path("v1"))
        .and(warp::path("groceries"))
        .and(warp::path::end())
        .and(json_body())
        .and(store_filter.clone())
        .and_then(add_grocery_list_item);

    let get_items = warp::get()
        .and(warp::path("v1"))
        .and(warp::path("groceries"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and_then(get_grocery_list);

    let chuck = warp::get()
        .and(warp::path("v1"))
        .and(warp::path("chuck"))
        .and(warp::path::end())
        .and(pool_filter.clone())
        .and_then(chuck);

    let routes = add_items.or(get_items).or(chuck);

    proper_rust::start_server(routes).await;
}
