use prometheus::{Counter, Histogram, Registry, TextEncoder, Encoder, HistogramOpts};
use lambda_models::LambdaError;

pub struct PrometheusMetrics {
    registry: Registry,
    invocations_total: Counter,
    errors_total: Counter,
    throttles_total: Counter,
    cold_starts_total: Counter,
    duration_ms: Histogram,
    init_duration_ms: Histogram,
}

impl PrometheusMetrics {
    pub fn new() -> Result<Self, LambdaError> {
        let registry = Registry::new();
        
        let invocations_total = Counter::new(
            "lambda_invocations_total",
            "Total number of Lambda invocations"
        ).map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        
        let errors_total = Counter::new(
            "lambda_errors_total",
            "Total number of Lambda errors"
        ).map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        
        let throttles_total = Counter::new(
            "lambda_throttles_total",
            "Total number of Lambda throttles"
        ).map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        
        let cold_starts_total = Counter::new(
            "lambda_cold_starts_total",
            "Total number of Lambda cold starts"
        ).map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        
        let duration_ms = Histogram::with_opts(HistogramOpts::new(
            "lambda_duration_ms",
            "Lambda function execution duration in milliseconds"
        )).map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        
        let init_duration_ms = Histogram::with_opts(HistogramOpts::new(
            "lambda_init_duration_ms",
            "Lambda function initialization duration in milliseconds"
        )).map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        
        // Register metrics
        registry.register(Box::new(invocations_total.clone()))
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        registry.register(Box::new(errors_total.clone()))
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        registry.register(Box::new(throttles_total.clone()))
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        registry.register(Box::new(cold_starts_total.clone()))
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        registry.register(Box::new(duration_ms.clone()))
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        registry.register(Box::new(init_duration_ms.clone()))
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        
        Ok(Self {
            registry,
            invocations_total,
            errors_total,
            throttles_total,
            cold_starts_total,
            duration_ms,
            init_duration_ms,
        })
    }

    pub fn record_invocation(&self, _function_name: &str) {
        self.invocations_total.inc();
    }

    pub fn record_error(&self, _function_name: &str, _error_type: &str) {
        self.errors_total.inc();
    }

    pub fn record_throttle(&self, _function_name: &str) {
        self.throttles_total.inc();
    }

    pub fn record_cold_start(&self, _function_name: &str) {
        self.cold_starts_total.inc();
    }

    pub fn record_duration(&self, _function_name: &str, duration_ms: f64) {
        self.duration_ms.observe(duration_ms);
    }

    pub fn record_init_duration(&self, _function_name: &str, duration_ms: f64) {
        self.init_duration_ms.observe(duration_ms);
    }

    pub fn get_metrics(&self) -> Result<String, LambdaError> {
        let metric_families = self.registry.gather();
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();
        
        encoder.encode(&metric_families, &mut buffer)
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        
        String::from_utf8(buffer)
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })
    }
}
