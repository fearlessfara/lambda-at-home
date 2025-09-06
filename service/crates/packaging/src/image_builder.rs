use crate::runtimes;
use crate::zip_handler::ZipInfo;
use lambda_models::{Function, LambdaError};
use rust_embed::RustEmbed;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{error, info, instrument};

#[derive(RustEmbed)]
#[folder = "../../runtimes"]
struct RuntimeAssets;

pub struct ImageBuilder {
    _docker_host: String,
}

/// Get embedded bootstrap file content for a given runtime
fn get_embedded_bootstrap(function: &Function) -> Result<Vec<u8>, LambdaError> {
    let bootstrap_path = match function.runtime.as_str() {
        "nodejs18.x" => "nodejs18/bootstrap.js",
        "nodejs22.x" => "nodejs22/bootstrap.js",
        "python3.11" => "python311/bootstrap.py",
        _ => return Err(LambdaError::InternalError {
            reason: format!("Unsupported runtime: {}", function.runtime),
        }),
    };
    
    RuntimeAssets::get(bootstrap_path)
        .ok_or_else(|| LambdaError::InternalError {
            reason: format!("Bootstrap file not found: {}", bootstrap_path),
        })
        .map(|file| file.data.into_owned())
}

impl ImageBuilder {
    pub fn new(docker_host: String) -> Self {
        Self {
            _docker_host: docker_host,
        }
    }

    #[instrument(skip(self, function, zip_info))]
    pub async fn build_image(
        &self,
        function: &Function,
        zip_info: &ZipInfo,
        image_ref: &str,
    ) -> Result<(), LambdaError> {
        // Create temporary directory for build context
        let temp_dir = tempfile::tempdir().map_err(|e| LambdaError::InternalError {
            reason: e.to_string(),
        })?;

        let build_context = temp_dir.path();

        // Extract ZIP to build context
        let zip_handler = crate::zip_handler::ZipHandler::new(50 * 1024 * 1024); // 50MB limit
        zip_handler
            .extract_to_directory(&zip_info.zip_data, build_context)
            .await?;

        // Copy embedded bootstrap script to build context
        if let Ok(bootstrap_content) = get_embedded_bootstrap(function) {
            let bootstrap_filename = match function.runtime.as_str() {
                "python3.11" => "bootstrap.py",
                _ => "bootstrap.js",
            };
            let bootstrap_dest = build_context.join(bootstrap_filename);
            std::fs::write(&bootstrap_dest, bootstrap_content).map_err(|e| {
                LambdaError::InternalError {
                    reason: e.to_string(),
                }
            })?;
        }

        // Create Dockerfile based on runtime
        let dockerfile_content = runtimes::dockerfile_for(function);
        let dockerfile_path = build_context.join("Dockerfile");
        std::fs::write(&dockerfile_path, dockerfile_content).map_err(|e| {
            LambdaError::InternalError {
                reason: e.to_string(),
            }
        })?;

        // Bootstrap files are already copied above using embedded assets

        // Build Docker image
        info!("Building Docker image: {}", image_ref);
        info!("Build context: {:?}", build_context);
        info!("Dockerfile path: {:?}", dockerfile_path);

        let build_result = Command::new("docker")
            .arg("build")
            .arg("-t")
            .arg(image_ref)
            .arg("-f")
            .arg(&dockerfile_path)
            .arg(build_context)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| LambdaError::DockerError {
                message: e.to_string(),
            })?;

        if !build_result.status.success() {
            let stdout = String::from_utf8_lossy(&build_result.stdout);
            let stderr = String::from_utf8_lossy(&build_result.stderr);
            error!("Docker build failed - stdout: {}", stdout);
            error!("Docker build failed - stderr: {}", stderr);
            return Err(LambdaError::DockerError {
                message: format!("Docker build failed: {}", stderr),
            });
        }

        info!("Built Docker image: {}", image_ref);
        Ok(())
    }
}
