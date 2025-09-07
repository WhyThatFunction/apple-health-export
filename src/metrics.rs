use once_cell::sync::Lazy;
use opentelemetry::metrics::{Counter, UpDownCounter};
use opentelemetry::{KeyValue, global};

// Centralized metrics instruments used across the app.
pub struct Metrics {
    ingest_requests_total: Counter<u64>,
    jobs_inflight: UpDownCounter<i64>,
}

static METRICS: Lazy<Metrics> = Lazy::new(|| {
    let meter = global::meter("apple-health-export");

    let ingest_requests_total = meter
        .u64_counter("ahe_ingest_requests_total")
        .with_description("Total number of ingest endpoint requests")
        .build();

    let jobs_inflight = meter
        .i64_up_down_counter("ahe_jobs_inflight")
        .with_description("Number of background jobs currently in-flight")
        .build();

    Metrics {
        ingest_requests_total,
        jobs_inflight,
    }
});

pub fn inc_ingest_request(method: &str, device_name: Option<&str>) {
    let mut attrs = vec![KeyValue::new("http.method", method.to_string())];
    if let Some(dev) = device_name {
        attrs.push(KeyValue::new("device.name", dev.to_string()));
    }
    METRICS.ingest_requests_total.add(1, &attrs);
}

pub fn inc_jobs_inflight() {
    METRICS.jobs_inflight.add(1, &[]);
}

pub fn dec_jobs_inflight() {
    METRICS.jobs_inflight.add(-1, &[]);
}
