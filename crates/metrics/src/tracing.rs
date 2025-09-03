use tracing::{info, error, warn};
use lambda_models::{Function, ExecutionStatus, ErrorType};

pub struct TracingService;

impl TracingService {
    pub fn init() -> Result<(), Box<dyn std::error::Error>> {
        tracing_subscriber::fmt()
            .json()
            .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
            .init();
        
        Ok(())
    }

    pub fn log_function_created(function: &Function) {
        info!(
            function_name = %function.function_name,
            function_id = %function.function_id,
            runtime = %function.runtime,
            handler = %function.handler,
            memory_size = function.memory_size,
            timeout = function.timeout,
            "Function created"
        );
    }

    pub fn log_function_deleted(function_name: &str) {
        info!(
            function_name = %function_name,
            "Function deleted"
        );
    }

    pub fn log_invocation_started(
        function_name: &str,
        request_id: &str,
        container_id: Option<&str>,
        is_cold_start: bool,
    ) {
        info!(
            function_name = %function_name,
            request_id = %request_id,
            container_id = %container_id.unwrap_or("none"),
            is_cold_start = is_cold_start,
            "Invocation started"
        );
    }

    pub fn log_invocation_completed(
        function_name: &str,
        request_id: &str,
        container_id: Option<&str>,
        duration_ms: u64,
        billed_ms: u64,
        memory_used_mb: Option<u64>,
        status: ExecutionStatus,
    ) {
        let level = match status {
            ExecutionStatus::Success => tracing::Level::INFO,
            ExecutionStatus::Error | ExecutionStatus::Timeout => tracing::Level::ERROR,
            _ => tracing::Level::WARN,
        };

        match level {
            tracing::Level::INFO => {
                info!(
                    function_name = %function_name,
                    request_id = %request_id,
                    container_id = %container_id.unwrap_or("none"),
                    duration_ms = duration_ms,
                    billed_ms = billed_ms,
                    memory_used_mb = memory_used_mb.unwrap_or(0),
                    status = %format!("{:?}", status),
                    "Invocation completed"
                );
            }
            tracing::Level::ERROR => {
                error!(
                    function_name = %function_name,
                    request_id = %request_id,
                    container_id = %container_id.unwrap_or("none"),
                    duration_ms = duration_ms,
                    billed_ms = billed_ms,
                    memory_used_mb = memory_used_mb.unwrap_or(0),
                    status = %format!("{:?}", status),
                    "Invocation failed"
                );
            }
            _ => {
                warn!(
                    function_name = %function_name,
                    request_id = %request_id,
                    container_id = %container_id.unwrap_or("none"),
                    duration_ms = duration_ms,
                    billed_ms = billed_ms,
                    memory_used_mb = memory_used_mb.unwrap_or(0),
                    status = %format!("{:?}", status),
                    "Invocation completed with warning"
                );
            }
        }
    }

    pub fn log_container_created(
        function_name: &str,
        container_id: &str,
        image_ref: &str,
        is_warm: bool,
    ) {
        info!(
            function_name = %function_name,
            container_id = %container_id,
            image_ref = %image_ref,
            is_warm = is_warm,
            "Container created"
        );
    }

    pub fn log_container_stopped(
        function_name: &str,
        container_id: &str,
        reason: &str,
    ) {
        info!(
            function_name = %function_name,
            container_id = %container_id,
            reason = %reason,
            "Container stopped"
        );
    }

    pub fn log_container_removed(
        function_name: &str,
        container_id: &str,
        reason: &str,
    ) {
        info!(
            function_name = %function_name,
            container_id = %container_id,
            reason = %reason,
            "Container removed"
        );
    }

    pub fn log_error(
        function_name: &str,
        request_id: Option<&str>,
        container_id: Option<&str>,
        error_type: ErrorType,
        error_message: &str,
    ) {
        error!(
            function_name = %function_name,
            request_id = %request_id.unwrap_or("none"),
            container_id = %container_id.unwrap_or("none"),
            error_type = %format!("{:?}", error_type),
            error_message = %error_message,
            "Error occurred"
        );
    }

    pub fn log_throttle(
        function_name: &str,
        request_id: Option<&str>,
        reason: &str,
    ) {
        warn!(
            function_name = %function_name,
            request_id = %request_id.unwrap_or("none"),
            reason = %reason,
            "Request throttled"
        );
    }

    pub fn log_idle_cleanup(
        containers_stopped: usize,
        containers_removed: usize,
    ) {
        info!(
            containers_stopped = containers_stopped,
            containers_removed = containers_removed,
            "Idle cleanup completed"
        );
    }
}
