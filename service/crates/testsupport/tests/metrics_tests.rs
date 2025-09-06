use lambda_testsupport::metrics::*;

#[test]
fn test_prom_parse() {
    let text = r#"
# HELP lambda_invocations_total Total number of Lambda invocations
# TYPE lambda_invocations_total counter
lambda_invocations_total 5
# HELP lambda_duration_ms Lambda function execution duration in milliseconds
# TYPE lambda_duration_ms histogram
lambda_duration_ms_bucket 0
lambda_duration_ms_bucket 0
lambda_duration_ms_bucket 5
lambda_duration_ms_sum 100.5
lambda_duration_ms_count 5
"#;

    let metrics = prom_parse(text).unwrap();
    assert_eq!(metrics.counters.get("lambda_invocations_total"), Some(&5.0));
    assert_eq!(metrics.counters.get("lambda_duration_ms_count"), Some(&5.0));

    let duration_hist = metrics.histograms.get("lambda_duration_ms").unwrap();
    assert_eq!(duration_hist.sum, 100.5);

    // The bucket names are extracted from the metric names, so they should be "unknown" for this test data
    // Since all buckets have the same key "unknown", the last value (5.0) overwrites the previous ones
    assert_eq!(duration_hist.buckets.get("unknown"), Some(&5.0));
}
