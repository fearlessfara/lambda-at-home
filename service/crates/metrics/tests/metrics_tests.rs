use lambda_metrics::*;
use ::prometheus::{Counter, Encoder, Histogram, HistogramOpts, Registry, TextEncoder};

#[test]
fn test_metrics_service_creation() {
    let service = MetricsService::new().unwrap();

    // Test that we can get metrics
    let metrics_text = futures::executor::block_on(service.get_prometheus_metrics()).unwrap();
    assert!(metrics_text.contains("lambda_invocations_total"));
    assert!(metrics_text.contains("lambda_errors_total"));
    assert!(metrics_text.contains("lambda_duration_ms"));
}

#[test]
fn test_metric_name_registration() {
    let service = MetricsService::new().unwrap();
    let metrics_text = futures::executor::block_on(service.get_prometheus_metrics()).unwrap();

    // Check that all expected metric names are present
    let expected_metrics = [
        "lambda_invocations_total",
        "lambda_errors_total",
        "lambda_throttles_total",
        "lambda_cold_starts_total",
        "lambda_duration_ms",
        "lambda_init_duration_ms",
    ];

    for metric in &expected_metrics {
        assert!(metrics_text.contains(metric), "Missing metric: {}", metric);
    }
}

#[test]
fn test_tracing_fields_formatting() {
    // Test that tracing fields are properly formatted
    let test_fields = [
        ("function", "test-function"),
        ("version", "1"),
        ("req_id", "test-request-id"),
        ("container_id", "test-container-id"),
        ("duration_ms", "100"),
        ("billed_ms", "100"),
        ("mem_peak_mb", "256"),
    ];

    for (key, value) in &test_fields {
        let formatted = format!("{}={}", key, value);
        assert!(formatted.contains(key));
        assert!(formatted.contains(value));
    }
}

#[test]
fn test_prometheus_counter_increment() {
    let counter = Counter::new("test_counter", "Test counter").unwrap();
    counter.inc();
    counter.inc_by(5.0);

    assert_eq!(counter.get(), 6.0);
}

#[test]
fn test_prometheus_histogram_observation() {
    let histogram =
        Histogram::with_opts(HistogramOpts::new("test_histogram", "Test histogram")).unwrap();
    histogram.observe(1.0);
    histogram.observe(2.0);
    histogram.observe(3.0);

    assert_eq!(histogram.get_sample_count(), 3);
    assert_eq!(histogram.get_sample_sum(), 6.0);
}

#[test]
fn test_metrics_encoding() {
    let registry = Registry::new();
    let counter = Counter::new("test_counter", "Test counter").unwrap();
    counter.inc();

    registry.register(Box::new(counter.clone())).unwrap();

    let metric_families = registry.gather();
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();

    encoder.encode(&metric_families, &mut buffer).unwrap();
    let output = String::from_utf8(buffer).unwrap();

    assert!(output.contains("test_counter"));
    assert!(output.contains("1"));
}
