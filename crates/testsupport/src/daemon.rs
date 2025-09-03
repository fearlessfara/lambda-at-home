use anyhow::Result;
use lambda_models::Config;
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use tokio::process::{Command, Child};
use tokio::time::sleep;

#[derive(Debug)]
pub struct TestDaemon {
    pub user_api_url: String,
    pub runtime_api_url: String,
    pub data_dir: TempDir,
    process: Child,
}

impl TestDaemon {
    pub async fn kill(&mut self) -> Result<()> {
        self.process.kill().await?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct ConfigOverride {
    pub idle_soft_ms: Option<u64>,
    pub idle_hard_ms: Option<u64>,
    pub max_global_concurrency: Option<u32>,
    pub port_user_api: Option<u16>,
    pub port_runtime_api: Option<u16>,
}

pub async fn spawn_daemon(config_override: Option<ConfigOverride>) -> Result<TestDaemon> {
    // Create temporary data directory
    let data_dir = tempfile::tempdir()?;
    let data_path = data_dir.path().to_string_lossy();
    
    // Create config file with overrides
    let mut config = Config::default();
    if let Some(override_config) = config_override {
        if let Some(soft_ms) = override_config.idle_soft_ms {
            config.idle.soft_ms = soft_ms;
        }
        if let Some(hard_ms) = override_config.idle_hard_ms {
            config.idle.hard_ms = hard_ms;
        }
        if let Some(max_concurrency) = override_config.max_global_concurrency {
            config.limits.max_global_concurrency = max_concurrency;
        }
        if let Some(port) = override_config.port_user_api {
            config.server.port_user_api = port;
        }
        if let Some(port) = override_config.port_runtime_api {
            config.server.port_runtime_api = port;
        }
    }
    
    // Override data directory
    config.data.dir = data_path.to_string();
    config.data.db_url = format!("sqlite://{}/lhome.db", data_path);
    
    // Start the daemon with environment variables
    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--bin", "lambda-at-home-server"]);
    cmd.env("LAMBDA_DATA_DIR", data_path.as_ref());
    cmd.env("LAMBDA_DB_URL", &config.data.db_url);
    cmd.env("LAMBDA_USER_API_PORT", config.server.port_user_api.to_string());
    cmd.env("LAMBDA_RUNTIME_API_PORT", config.server.port_runtime_api.to_string());
    cmd.env("LAMBDA_IDLE_SOFT_MS", config.idle.soft_ms.to_string());
    cmd.env("LAMBDA_IDLE_HARD_MS", config.idle.hard_ms.to_string());
    cmd.env("LAMBDA_MAX_GLOBAL_CONCURRENCY", config.limits.max_global_concurrency.to_string());
    cmd.current_dir(std::env::current_dir()?);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    
    let process = cmd.spawn()?;
    
    // Wait for daemon to start
    sleep(Duration::from_secs(3)).await;
    
    let user_api_url = format!("http://{}:{}", config.server.bind, config.server.port_user_api);
    let runtime_api_url = format!("http://{}:{}", config.server.bind, config.server.port_runtime_api);
    
    Ok(TestDaemon {
        user_api_url,
        runtime_api_url,
        data_dir,
        process,
    })
}
