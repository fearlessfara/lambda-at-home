
use std::process::Stdio;
use tokio::process::Command;
use lambda_models::{Function, LambdaError};
use crate::runtimes;
use crate::zip_handler::ZipInfo;
use tracing::{info, error, instrument};

pub struct ImageBuilder {
    _docker_host: String,
}

impl ImageBuilder {
    pub fn new(docker_host: String) -> Self {
        Self { _docker_host: docker_host }
    }

    #[instrument(skip(self, function, zip_info))]
    pub async fn build_image(&self, function: &Function, zip_info: &ZipInfo, image_ref: &str) -> Result<(), LambdaError> {
        
        // Create temporary directory for build context
        let temp_dir = tempfile::tempdir()
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        
        let build_context = temp_dir.path();
        
        // Extract ZIP to build context
        let zip_handler = crate::zip_handler::ZipHandler::new(50 * 1024 * 1024); // 50MB limit
        zip_handler.extract_to_directory(&zip_info.zip_data, build_context).await?;
        
        // Copy bootstrap script to build context based on runtime
        let runtime_dir = match function.runtime.as_str() {
            "nodejs22.x" => "nodejs22",
            _ => "nodejs18",
        };
        let bootstrap_source = std::env::current_dir()
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?
            .join("runtimes")
            .join(runtime_dir)
            .join("bootstrap.js");
        
        if bootstrap_source.exists() {
            let bootstrap_dest = build_context.join("bootstrap.js");
            std::fs::copy(&bootstrap_source, &bootstrap_dest)
                .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        }
        
        // Create Dockerfile based on runtime
        let dockerfile_content = runtimes::dockerfile_for(function);
        let dockerfile_path = build_context.join("Dockerfile");
        std::fs::write(&dockerfile_path, dockerfile_content)
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        
        // Copy runtime bootstrap, if any
        if let Some((rel, dest_name)) = runtimes::bootstrap_source(function) {
            let abs = std::env::current_dir()
                .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?
                .join(rel);
            if abs.exists() {
                let dst = build_context.join(dest_name);
                std::fs::copy(&abs, &dst)
                    .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
            }
        }

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
            .map_err(|e| LambdaError::DockerError { message: e.to_string() })?;
        
        if !build_result.status.success() {
            let stdout = String::from_utf8_lossy(&build_result.stdout);
            let stderr = String::from_utf8_lossy(&build_result.stderr);
            error!("Docker build failed - stdout: {}", stdout);
            error!("Docker build failed - stderr: {}", stderr);
            return Err(LambdaError::DockerError { 
                message: format!("Docker build failed: {}", stderr) 
            });
        }
        
        info!("Built Docker image: {}", image_ref);
        Ok(())
    }
}
