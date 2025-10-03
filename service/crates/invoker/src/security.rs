use lambda_models::LambdaError;
use tracing::{info, warn};

pub struct SecurityConfig {
    pub drop_capabilities: bool,
    pub read_only_rootfs: bool,
    pub no_new_privileges: bool,
    pub user_id: u32,
    pub group_id: u32,
    pub memory_limit_mb: u64,
    pub cpu_quota: i64,
    pub pids_limit: i64,
    pub tmpfs_size_mb: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            drop_capabilities: true,
            read_only_rootfs: true,
            no_new_privileges: true,
            user_id: 1000,
            group_id: 1000,
            memory_limit_mb: 512,
            cpu_quota: 100000, // 1 CPU core
            pids_limit: 1024,
            tmpfs_size_mb: 512,
        }
    }
}

impl SecurityConfig {
    pub fn validate(&self) -> Result<(), LambdaError> {
        if self.memory_limit_mb == 0 {
            return Err(LambdaError::InvalidRequest { 
                reason: "Memory limit must be greater than 0".to_string() 
            });
        }
        
        if self.cpu_quota <= 0 {
            return Err(LambdaError::InvalidRequest { 
                reason: "CPU quota must be greater than 0".to_string() 
            });
        }
        
        if self.pids_limit <= 0 {
            return Err(LambdaError::InvalidRequest { 
                reason: "PIDs limit must be greater than 0".to_string() 
            });
        }
        
        if self.tmpfs_size_mb == 0 {
            return Err(LambdaError::InvalidRequest { 
                reason: "Tmpfs size must be greater than 0".to_string() 
            });
        }
        
        Ok(())
    }
    
    pub fn log_security_settings(&self) {
        info!("Security configuration:");
        info!("  Drop capabilities: {}", self.drop_capabilities);
        info!("  Read-only rootfs: {}", self.read_only_rootfs);
        info!("  No new privileges: {}", self.no_new_privileges);
        info!("  User ID: {}", self.user_id);
        info!("  Group ID: {}", self.group_id);
        info!("  Memory limit: {} MB", self.memory_limit_mb);
        info!("  CPU quota: {}", self.cpu_quota);
        info!("  PIDs limit: {}", self.pids_limit);
        info!("  Tmpfs size: {} MB", self.tmpfs_size_mb);
    }
}

pub fn validate_image_reference(image_ref: &str) -> Result<(), LambdaError> {
    // Basic validation of image reference
    if image_ref.is_empty() {
        return Err(LambdaError::InvalidRequest { 
            reason: "Image reference cannot be empty".to_string() 
        });
    }
    
    // Check for allowed image prefixes (security measure)
    let allowed_prefixes = [
        "lambda-home/",
        "node:",
        "python:",
        "alpine:",
    ];
    
    let is_allowed = allowed_prefixes.iter().any(|prefix| image_ref.starts_with(prefix));
    
    if !is_allowed {
        warn!("Potentially unsafe image reference: {}", image_ref);
        // For now, we'll allow it but log a warning
        // In production, you might want to be more restrictive
    }
    
    Ok(())
}

pub fn sanitize_environment_variables(env_vars: &mut std::collections::HashMap<String, String>, runtime_api_port: u16) {
    // Remove potentially dangerous environment variables
    let dangerous_vars = [
        "PATH",
        "LD_LIBRARY_PATH",
        "LD_PRELOAD",
        "PYTHONPATH",
        "NODE_PATH",
        "HOME",
        "USER",
        "SHELL",
    ];

    for var in &dangerous_vars {
        env_vars.remove(*var);
    }

    // Ensure AWS_LAMBDA_* variables are properly set
    if !env_vars.contains_key("AWS_LAMBDA_RUNTIME_API") {
        env_vars.insert("AWS_LAMBDA_RUNTIME_API".to_string(), format!("host.docker.internal:{}", runtime_api_port));
    }

    if !env_vars.contains_key("TZ") {
        env_vars.insert("TZ".to_string(), "UTC".to_string());
    }
}
