use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, CounterVec, Encoder,
    GaugeVec, HistogramVec, TextEncoder,
};

lazy_static! {
    // Request counters
    pub static ref REQUEST_COUNTER: CounterVec = register_counter_vec!(
        "llm_gateway_requests_total",
        "Total number of requests by model and provider",
        &["model", "provider", "status"]
    )
    .unwrap();

    // Token counters
    pub static ref TOKEN_COUNTER: CounterVec = register_counter_vec!(
        "llm_gateway_tokens_total",
        "Total number of tokens processed by model and type",
        &["model", "provider", "token_type"]
    )
    .unwrap();

    // Cost counter
    pub static ref COST_COUNTER: CounterVec = register_counter_vec!(
        "llm_gateway_cost_usd_total",
        "Total cost in USD by model and provider",
        &["model", "provider"]
    )
    .unwrap();

    // Cache hit/miss counter
    pub static ref CACHE_COUNTER: CounterVec = register_counter_vec!(
        "llm_gateway_cache_total",
        "Cache hits and misses",
        &["status"]
    )
    .unwrap();

    // Request latency histogram
    pub static ref REQUEST_LATENCY: HistogramVec = register_histogram_vec!(
        "llm_gateway_request_duration_seconds",
        "Request latency in seconds by model and provider",
        &["model", "provider"],
        vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .unwrap();

    // Active requests gauge
    pub static ref ACTIVE_REQUESTS: GaugeVec = register_gauge_vec!(
        "llm_gateway_active_requests",
        "Number of active requests by provider",
        &["provider"]
    )
    .unwrap();

    // Model availability gauge
    pub static ref MODEL_AVAILABILITY: GaugeVec = register_gauge_vec!(
        "llm_gateway_model_available",
        "Model availability (1 = available, 0 = unavailable)",
        &["model", "provider"]
    )
    .unwrap();

    // Rate limit metrics
    pub static ref RATE_LIMIT_EXCEEDED: CounterVec = register_counter_vec!(
        "llm_gateway_rate_limit_exceeded_total",
        "Total number of requests that exceeded rate limits",
        &["key_id", "limit_type"]
    )
    .unwrap();

    pub static ref RATE_LIMIT_REMAINING: GaugeVec = register_gauge_vec!(
        "llm_gateway_rate_limit_remaining",
        "Remaining capacity for rate limits",
        &["key_id", "limit_type"]
    )
    .unwrap();
}

pub struct MetricsCollector;

impl MetricsCollector {
    pub fn record_request(model: &str, provider: &str, success: bool) {
        let status = if success { "success" } else { "error" };
        REQUEST_COUNTER
            .with_label_values(&[model, provider, status])
            .inc();
    }

    pub fn record_tokens(model: &str, provider: &str, prompt_tokens: i32, completion_tokens: i32) {
        TOKEN_COUNTER
            .with_label_values(&[model, provider, "prompt"])
            .inc_by(prompt_tokens as f64);
        TOKEN_COUNTER
            .with_label_values(&[model, provider, "completion"])
            .inc_by(completion_tokens as f64);
    }

    pub fn record_cost(model: &str, provider: &str, cost_usd: f64) {
        COST_COUNTER
            .with_label_values(&[model, provider])
            .inc_by(cost_usd);
    }

    pub fn record_cache_hit() {
        CACHE_COUNTER.with_label_values(&["hit"]).inc();
    }

    pub fn record_cache_miss() {
        CACHE_COUNTER.with_label_values(&["miss"]).inc();
    }

    pub fn record_latency(model: &str, provider: &str, latency_seconds: f64) {
        REQUEST_LATENCY
            .with_label_values(&[model, provider])
            .observe(latency_seconds);
    }

    pub fn inc_active_requests(provider: &str) {
        ACTIVE_REQUESTS.with_label_values(&[provider]).inc();
    }

    pub fn dec_active_requests(provider: &str) {
        ACTIVE_REQUESTS.with_label_values(&[provider]).dec();
    }

    pub fn set_model_availability(model: &str, provider: &str, available: bool) {
        let value = if available { 1.0 } else { 0.0 };
        MODEL_AVAILABILITY
            .with_label_values(&[model, provider])
            .set(value);
    }

    pub fn record_rate_limit_exceeded(key_id: &str, limit_type: &str) {
        RATE_LIMIT_EXCEEDED
            .with_label_values(&[key_id, limit_type])
            .inc();
    }

    pub fn set_rate_limit_remaining(key_id: &str, limit_type: &str, remaining: i32) {
        RATE_LIMIT_REMAINING
            .with_label_values(&[key_id, limit_type])
            .set(remaining as f64);
    }

    pub fn export_metrics() -> Result<String, Box<dyn std::error::Error>> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}
