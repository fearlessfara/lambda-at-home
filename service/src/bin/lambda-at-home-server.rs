use anyhow::Result;
use lambda_control::ControlPlane;
use lambda_control::IdleWatchdog;
use lambda_invoker::Invoker;
use lambda_metrics::MetricsService;
use lambda_models::Config;
use sqlx::SqlitePool;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};

fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    // Try to load from various config locations
    let config_paths = [
        "service/configs/default.toml",
        "configs/default.toml",
        "config/config.toml",
    ];

    for path in &config_paths {
        if Path::new(path).exists() {
            let mut file = fs::File::open(path)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            let config: Config = toml::from_str(&contents)?;
            return Ok(config);
        }
    }

    Err("No config file found".into())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().init();

    // Change to the root directory if we're running from service/
    if std::env::current_dir()?.ends_with("service") {
        std::env::set_current_dir("..")?;
    }

    info!("Starting Lambda@Home server");

    // Load configuration from file or use defaults
    let config = load_config().unwrap_or_else(|e| {
        warn!("Failed to load config file: {}, using defaults", e);
        Config::default()
    });

    info!("Configuration loaded: {:?}", config);

    // Ensure data directory and DB parent directory exist when using SQLite
    // Handle paths relative to the current working directory
    let data_dir = if config.data.dir.starts_with("service/") {
        config.data.dir.clone()
    } else {
        format!("service/{}", config.data.dir)
    };

    if !data_dir.is_empty() {
        let _ = fs::create_dir_all(&data_dir);
    }

    let db_url = if config.data.db_url.starts_with("sqlite://service/") {
        config.data.db_url.clone()
    } else if config.data.db_url.starts_with("sqlite://") {
        format!("sqlite://service/{}", &config.data.db_url[9..])
    } else {
        config.data.db_url.clone()
    };

    // Handle both sqlite:// and sqlite: formats
    let db_path = if let Some(path) = db_url.strip_prefix("sqlite://") {
        Some(path)
    } else {
        db_url.strip_prefix("sqlite:")
    };

    if let Some(db_path) = db_path {
        if let Some(parent) = Path::new(db_path).parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                warn!("Failed to create DB parent directory {:?}: {}", parent, e);
            }
        }
        // Create the database file if it doesn't exist
        if !Path::new(db_path).exists() {
            if let Err(e) = fs::File::create(db_path) {
                warn!("Failed to create database file {:?}: {}", db_path, e);
            } else {
                info!("Created database file: {}", db_path);
            }
        }
    }

    // Initialize database pool
    let pool = SqlitePool::connect(&db_url).await?;
    info!("Database connected");

    // Initialize metrics service
    let metrics = Arc::new(MetricsService::new()?);

    // Initialize invoker
    let invoker = Arc::new(Invoker::new(config.clone()).await?);

    // Initialize control plane
    let control_plane = Arc::new(ControlPlane::new(pool, invoker, config.clone()).await?);

    // Start the control plane (no start method needed for now)
    let control_handle = {
        let _control_plane = control_plane.clone();
        tokio::spawn(async move {
            // Control plane is ready to handle requests
            info!("Control plane initialized and ready");
            // Keep the task alive
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        })
    };

    // Start idle watchdog
    let watchdog_handle = {
        let cp = control_plane.clone();
        tokio::spawn(async move {
            let watchdog = IdleWatchdog::new(
                cp.config(),
                cp.warm_pool(),
                Arc::new(cp.pending()),
                cp.invoker(),
            );
            watchdog.start().await;
        })
    };

    // Clone config values for the servers
    let bind_addr = config.server.bind.clone();
    let user_api_port = config.server.port_user_api;
    let runtime_api_port = config.server.port_runtime_api;

    // Start user API server
    let user_api_handle = {
        let control_plane = control_plane.clone();
        let metrics = metrics.clone();
        let bind = bind_addr.clone();
        let config = config.clone();
        tokio::spawn(async move {
            if let Err(e) =
                lambda_api::start_server(bind, user_api_port, control_plane, metrics, config).await
            {
                warn!("User API server error: {}", e);
            }
        })
    };

    // Start runtime API server
    let runtime_api_handle = {
        let control_plane = control_plane.clone();
        let bind = bind_addr.clone();
        tokio::spawn(async move {
            if let Err(e) =
                lambda_runtime_api::start_server(bind, runtime_api_port, control_plane).await
            {
                warn!("Runtime API server error: {}", e);
            }
        })
    };

    info!(
        "Lambda@Home server started successfully. User API: {}:{}, Runtime API: {}:{}",
        bind_addr, user_api_port, bind_addr, runtime_api_port
    );

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal");
        }
        Err(err) => {
            warn!("Unable to listen for shutdown signal: {}", err);
        }
    }

    // Graceful shutdown
    info!("Shutting down Lambda@Home server...");

    // Cancel all tasks
    control_handle.abort();
    user_api_handle.abort();
    runtime_api_handle.abort();
    watchdog_handle.abort();

    // Best-effort: remove any remaining containers
    {
        let inv = control_plane.invoker();
        let pool = control_plane.warm_pool();
        let ids = pool.drain_all().await;
        for id in ids {
            let _ = inv.remove_container(&id).await;
        }
    }

    // Wait a bit for graceful shutdown
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("Lambda@Home server shutdown complete");
    Ok(())
}
