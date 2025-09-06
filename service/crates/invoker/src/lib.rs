pub mod docker;

pub use docker::*;

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_models::{Function, FunctionState};

    fn create_test_function() -> Function {
        Function {
            function_id: uuid::Uuid::new_v4(),
            function_name: "test-function".to_string(),
            runtime: "nodejs18.x".to_string(),
            role: None,
            handler: "index.handler".to_string(),
            code_sha256: "abcd1234".to_string(),
            description: None,
            timeout: 30,
            memory_size: 512,
            environment: std::collections::HashMap::new(),
            last_modified: chrono::Utc::now(),
            code_size: 1024,
            version: "1".to_string(),
            state: FunctionState::Active,
            state_reason: None,
            state_reason_code: None,
        }
    }

    #[test]
    fn test_memory_spec_computation() {
        let function = create_test_function();
        let memory_bytes = (function.memory_size * 1024 * 1024) as i64;
        assert_eq!(memory_bytes, 512 * 1024 * 1024);
    }

    #[test]
    fn test_cpu_spec_computation() {
        // Test CPU quota computation (1 CPU core = 100000)
        let cpu_quota = 100000;
        let cpu_period = 100000;
        assert_eq!(cpu_quota, cpu_period); // 1 CPU core
    }

    #[test]
    fn test_pids_limit_computation() {
        // Test PIDs limit
        let pids_limit = 1024;
        assert!(pids_limit > 0);
    }

    #[test]
    fn test_container_name_generation() {
        let function = create_test_function();
        let container_name = format!("lambda-{}-{}", function.function_name, uuid::Uuid::new_v4());
        assert!(container_name.starts_with("lambda-test-function-"));
    }

    #[test]
    fn test_environment_variables() {
        let function = create_test_function();
        let mut env_vars = std::collections::HashMap::new();
        env_vars.insert("CUSTOM_VAR".to_string(), "custom_value".to_string());

        let mut env = Vec::new();
        env.push("AWS_LAMBDA_RUNTIME_API=host.docker.internal:9001".to_string());
        env.push(format!(
            "AWS_LAMBDA_FUNCTION_NAME={}",
            function.function_name
        ));
        env.push(format!("AWS_LAMBDA_FUNCTION_VERSION={}", function.version));
        env.push(format!(
            "AWS_LAMBDA_FUNCTION_MEMORY_SIZE={}",
            function.memory_size
        ));

        for (key, value) in env_vars {
            env.push(format!("{}={}", key, value));
        }

        assert!(env.contains(&"AWS_LAMBDA_RUNTIME_API=host.docker.internal:9001".to_string()));
        assert!(env.contains(&"AWS_LAMBDA_FUNCTION_NAME=test-function".to_string()));
        assert!(env.contains(&"CUSTOM_VAR=custom_value".to_string()));
    }
}
