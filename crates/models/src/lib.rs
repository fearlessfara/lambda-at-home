pub mod config;
pub mod function;
pub mod invoke;
pub mod error;
pub mod routes;
pub mod secrets;

pub use config::*;
pub use function::*;
pub use invoke::*;
pub use error::*;
pub use routes::*;
pub use secrets::*;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_function_serde_roundtrip() {
        let function = Function {
            function_id: uuid::Uuid::new_v4(),
            function_name: "test-function".to_string(),
            runtime: "nodejs18.x".to_string(),
            role: Some("arn:aws:iam::123456789012:role/lambda-role".to_string()),
            handler: "index.handler".to_string(),
            code_sha256: "abcd1234".to_string(),
            description: Some("Test function".to_string()),
            timeout: 30,
            memory_size: 512,
            environment: std::collections::HashMap::new(),
            last_modified: chrono::Utc::now(),
            code_size: 1024,
            version: "1".to_string(),
            state: FunctionState::Active,
            state_reason: None,
            state_reason_code: None,
        };

        let json = serde_json::to_string(&function).unwrap();
        let deserialized: Function = serde_json::from_str(&json).unwrap();
        assert_eq!(function.function_name, deserialized.function_name);
        assert_eq!(function.runtime, deserialized.runtime);
        assert_eq!(function.handler, deserialized.handler);
    }

    #[test]
    fn test_create_function_request_deny_unknown_fields() {
        let json = r#"{
            "FunctionName": "test",
            "Runtime": "nodejs18.x",
            "Handler": "index.handler",
            "Code": {"ZipFile": "dGVzdA=="},
            "UnknownField": "should_fail"
        }"#;

        let result: Result<CreateFunctionRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown field"));
    }

    #[test]
    fn test_invoke_request_serde() {
        let request = InvokeRequest {
            function_name: "test-function".to_string(),
            invocation_type: InvocationType::RequestResponse,
            log_type: Some(LogType::Tail),
            client_context: None,
            payload: Some(serde_json::json!({"test": "data"})),
            qualifier: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: InvokeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.function_name, deserialized.function_name);
        assert_eq!(request.invocation_type, deserialized.invocation_type);
    }

    #[test]
    fn test_invocation_type_from_str() {
        assert_eq!("RequestResponse".parse::<InvocationType>().unwrap(), InvocationType::RequestResponse);
        assert_eq!("Event".parse::<InvocationType>().unwrap(), InvocationType::Event);
        assert_eq!("DryRun".parse::<InvocationType>().unwrap(), InvocationType::DryRun);
        assert!("Invalid".parse::<InvocationType>().is_err());
    }

    #[test]
    fn test_log_type_from_str() {
        assert_eq!("None".parse::<LogType>().unwrap(), LogType::None);
        assert_eq!("Tail".parse::<LogType>().unwrap(), LogType::Tail);
        assert!("Invalid".parse::<LogType>().is_err());
    }

    #[test]
    fn test_error_shape_serde() {
        let error = ErrorShape {
            error_message: "Test error".to_string(),
            error_type: "TestError".to_string(),
            stack_trace: Some(vec!["line1".to_string(), "line2".to_string()]),
        };

        let json = serde_json::to_string(&error).unwrap();
        let deserialized: ErrorShape = serde_json::from_str(&json).unwrap();
        assert_eq!(error.error_message, deserialized.error_message);
        assert_eq!(error.error_type, deserialized.error_type);
    }
}
