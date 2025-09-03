// Core modules
pub mod core {
    pub mod config;
    pub mod models;
    pub mod storage;
}

// Docker integration modules
pub mod docker {
    pub mod docker;
    pub mod container_lifecycle;
    pub mod ric_docker;
}

// API modules
pub mod api {
    pub mod lambda_executor;
    pub mod lambda_runtime_api;
}

// CLI module
pub mod cli {
    pub mod cli;
}

// Re-export commonly used items for convenience
pub use core::config::Config;
pub use core::models::*;
pub use core::storage::FunctionStorage;
pub use docker::docker::DockerManager;
pub use docker::container_lifecycle::ContainerLifecycleManager;
pub use api::lambda_runtime_api::LambdaRuntimeService;
