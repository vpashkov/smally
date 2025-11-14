use once_cell::sync::Lazy;
use prometheus::{register_counter_vec, register_histogram, CounterVec, Histogram};

pub static REQUEST_COUNT: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "smally_requests_total",
        "Total number of embedding requests",
        &["status", "cached"]
    )
    .unwrap()
});

pub static REQUEST_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "smally_request_latency_seconds",
        "Request latency in seconds",
        vec![0.001, 0.005, 0.01, 0.02, 0.05, 0.1, 0.5, 1.0]
    )
    .unwrap()
});

pub static INFERENCE_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "smally_inference_latency_seconds",
        "Model inference latency in seconds",
        vec![0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1]
    )
    .unwrap()
});

pub static CACHE_HITS: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "smally_cache_hits_total",
        "Total number of cache hits",
        &["cache_level"]
    )
    .unwrap()
});

pub static CACHE_MISSES: Lazy<prometheus::Counter> = Lazy::new(|| {
    prometheus::register_counter!("smally_cache_misses_total", "Total number of cache misses")
        .unwrap()
});

pub static TOKEN_COUNT: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "smally_token_count",
        "Number of tokens in requests",
        vec![1.0, 5.0, 10.0, 20.0, 50.0, 100.0, 128.0]
    )
    .unwrap()
});

pub static ERROR_COUNT: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "smally_errors_total",
        "Total number of errors",
        &["error_type"]
    )
    .unwrap()
});

pub static RATE_LIMIT_EXCEEDED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "smally_rate_limit_exceeded_total",
        "Total number of rate limit exceeded errors",
        &["tier"]
    )
    .unwrap()
});
