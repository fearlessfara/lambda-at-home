use lambda_control::ControlPlane;
use lambda_invoker::Invoker;
use lambda_metrics::MetricsService;
use lambda_models::Config;
use lambda_packaging::PackagingService;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub control: Arc<ControlPlane>,
    pub invoker: Arc<Invoker>,
    pub packaging: Arc<PackagingService>,
    pub metrics: Arc<MetricsService>,
}

impl AppState {
    pub fn new(
        config: Config,
        control: Arc<ControlPlane>,
        invoker: Arc<Invoker>,
        packaging: Arc<PackagingService>,
        metrics: Arc<MetricsService>,
    ) -> Self {
        Self {
            config,
            control,
            invoker,
            packaging,
            metrics,
        }
    }
}
