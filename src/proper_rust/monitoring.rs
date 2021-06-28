use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::SystemTime;

use lazy_static::lazy_static;
use parking_lot::RwLock;
use prometheus::{Counter, Encoder, Opts, Registry, TextEncoder};
use prometheus::core::{AtomicF64, GenericCounter};

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

fn inc_metric(metric_name: &str, success: bool, value: f64) {
    let key = format!("{}_{}", metric_name, success);
    if !COUNTERS.counters.read().contains_key(key.as_str()) {
        let counter_opts = Opts::new(metric_name, metric_name.to_string() + " help");
        let mut labels = HashMap::new();
        let outcome = if success { "success" } else { "error" };
        labels.insert("outcome".to_string(), outcome.to_string());
        let with_labels = counter_opts.const_labels(labels);
        let counter = Counter::with_opts(with_labels).unwrap();
        COUNTERS.registry.read().register(Box::new(counter.clone())).unwrap();
        COUNTERS.counters.write().insert(key.to_string(), counter);
    }

    COUNTERS.counters.read().get(key.as_str()).unwrap().inc_by(value);
}

pub async fn timed<F, T, E>(name: &str, f: impl FnOnce() -> F) -> Result<T, E>
    where F: Future<Output=Result<T, E>>,
{
    let start = SystemTime::now();
    let res = f().await;
    let duration = start.elapsed().unwrap();

    match res {
        Ok(t) => {
            inc_metric(format!("{}_total", name).as_str(), true, 1.0);
            inc_metric(format!("{}_time_seconds", name).as_str(), true, duration.as_secs_f64());
            Ok(t)
        }
        Err(e) => {
            inc_metric(format!("{}_total", name).as_str(), false, 1.0);
            Err(e)
        }
    }
}

pub fn metrics() -> String {
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = COUNTERS.registry.read().gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}