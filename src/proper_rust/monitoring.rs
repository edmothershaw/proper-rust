use std::collections::HashMap;
use std::sync::Arc;

use deadpool_postgres::{Config, Manager, ManagerConfig, Pool, RecyclingMethod};
use futures::{AsyncReadExt, join};
use lazy_static::lazy_static;
use log::{error, info, Record};
use log4rs;
use log_mdc;
use parking_lot::RwLock;
use prometheus::{Counter, Encoder, Opts, Registry, TextEncoder};
use prometheus::{IntCounter, IntCounterVec, IntGauge, IntGaugeVec};
use prometheus::{
    register_int_counter, register_int_counter_vec, register_int_gauge, register_int_gauge_vec,
};
use prometheus::core::{AtomicF64, AtomicU64, GenericCounter, GenericGaugeVec};

type Counters = HashMap<String, GenericCounter<AtomicF64>>;

#[derive(Clone)]
struct MetricStore {
    registry: Arc<RwLock<Registry>>,
    counters: Arc<RwLock<Counters>>,
}

impl MetricStore {
    fn new() -> Self {
        MetricStore {
            registry: Arc::new(RwLock::new(Registry::new())),
            counters: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}


lazy_static! {
    static ref COUNTERS: MetricStore = MetricStore::new();
}

fn inc_metric(metric_name: &str) {
    if !COUNTERS.counters.read().contains_key(metric_name) {
        let counter_opts = Opts::new(metric_name, metric_name.to_string() + " help");
        let counter = Counter::with_opts(counter_opts).unwrap();
        COUNTERS.registry.read().register(Box::new(counter.clone())).unwrap();
        COUNTERS.counters.write().insert(metric_name.to_string(), counter);
    }

    COUNTERS.counters.read().get(metric_name).unwrap().inc();
}

pub fn timed<T, E>(name: &str, func: &dyn Fn() -> Result<T, E>) -> Result<T, E> {
    inc_metric(name);
    func()
}

pub fn metrics() -> String {
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = COUNTERS.registry.read().gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}