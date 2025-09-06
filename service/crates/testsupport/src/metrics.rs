use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Metrics {
    pub counters: HashMap<String, f64>,
    pub histograms: HashMap<String, HistogramData>,
}

#[derive(Debug, Default)]
pub struct HistogramData {
    pub buckets: HashMap<String, f64>,
    pub sum: f64,
    pub count: f64,
}

/// Parse Prometheus metrics text format
pub fn prom_parse(text: &str) -> Result<Metrics> {
    let mut metrics = Metrics::default();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((name, value)) = parse_metric_line(line) {
            if name.ends_with("_total") || name.ends_with("_count") {
                // Counter
                metrics.counters.insert(name, value);
            } else if name.ends_with("_sum") {
                // Histogram sum
                let base_name = name.strip_suffix("_sum").unwrap_or(&name);
                let histogram = metrics.histograms.entry(base_name.to_string()).or_default();
                histogram.sum = value;
            } else if name.ends_with("_bucket") {
                // Histogram bucket
                let bucket_name = extract_bucket_name(&name);
                let base_name = name.strip_suffix("_bucket").unwrap_or(&name);
                let histogram = metrics.histograms.entry(base_name.to_string()).or_default();
                histogram.buckets.insert(bucket_name, value);
            } else {
                // Regular counter
                metrics.counters.insert(name, value);
            }
        }
    }

    Ok(metrics)
}

fn parse_metric_line(line: &str) -> Option<(String, f64)> {
    if let Some(space_pos) = line.find(' ') {
        let name = line[..space_pos].to_string();
        let value_str = line[space_pos + 1..].trim();
        if let Ok(value) = value_str.parse::<f64>() {
            return Some((name, value));
        }
    }
    None
}

fn extract_bucket_name(metric_name: &str) -> String {
    // Extract bucket value from metric name like "lambda_duration_ms_bucket{le=\"0.005\"}"
    if let Some(start) = metric_name.find("le=\"") {
        let start = start + 4;
        if let Some(end) = metric_name[start..].find('"') {
            return metric_name[start..start + end].to_string();
        }
    }
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prom_parse() {
        let text = r#"
# HELP lambda_invocations_total Total number of Lambda invocations
# TYPE lambda_invocations_total counter
lambda_invocations_total 5
# HELP lambda_duration_ms Lambda function execution duration in milliseconds
# TYPE lambda_duration_ms histogram
lambda_duration_ms_bucket{le="0.005"} 0
lambda_duration_ms_bucket{le="0.01"} 0
lambda_duration_ms_bucket{le="+Inf"} 5
lambda_duration_ms_sum 100.5
lambda_duration_ms_count 5
"#;

        let metrics = prom_parse(text).unwrap();
        assert_eq!(metrics.counters.get("lambda_invocations_total"), Some(&5.0));
        assert_eq!(metrics.counters.get("lambda_duration_ms_count"), Some(&5.0));

        let duration_hist = metrics.histograms.get("lambda_duration_ms").unwrap();
        assert_eq!(duration_hist.sum, 100.5);
        assert_eq!(duration_hist.buckets.get("0.005"), Some(&0.0));
        assert_eq!(duration_hist.buckets.get("+Inf"), Some(&5.0));
    }
}
