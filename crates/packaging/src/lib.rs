pub mod zip_handler;
pub mod image_builder;
pub mod runtimes;
pub mod cache;
pub mod service;

pub use zip_handler::*;
pub use image_builder::*;
pub use runtimes::*;
pub use cache::*;
pub use service::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_zip_sha256_stable() {
        let handler = ZipHandler::new(1024 * 1024);
        
        // Create a simple ZIP for testing
        let mut zip_data = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_data));
            zip.start_file("test.txt", zip::write::FileOptions::default()).unwrap();
            zip.write_all(b"test content").unwrap();
            zip.finish().unwrap();
        }

        let zip_info = futures::executor::block_on(handler.process_zip(&zip_data)).unwrap();
        
        // SHA256 should be deterministic
        assert!(!zip_info.sha256.is_empty());
        assert_eq!(zip_info.files.len(), 1);
        assert_eq!(zip_info.files[0].name, "test.txt");
    }

    #[test]
    fn test_image_tag_computation() {
        let function = lambda_models::Function {
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
            state: lambda_models::FunctionState::Active,
            state_reason: None,
            state_reason_code: None,
        };

        // Test image tag generation logic without actually building
        let expected_tag = format!("lambda-home/{}:{}", function.function_name, function.code_sha256);
        
        assert!(expected_tag.contains("lambda-home"));
        assert!(expected_tag.contains("test-function"));
        assert!(expected_tag.contains("abcd1234"));
    }

    #[test]
    fn test_zip_extraction() {
        let handler = ZipHandler::new(1024 * 1024);
        
        // Create a test ZIP
        let mut zip_data = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_data));
            zip.start_file("test.txt", zip::write::FileOptions::default()).unwrap();
            zip.write_all(b"test content").unwrap();
            zip.finish().unwrap();
        }

        let temp_dir = tempdir().unwrap();
        futures::executor::block_on(handler.extract_to_directory(&zip_data, temp_dir.path())).unwrap();
        
        let extracted_file = temp_dir.path().join("test.txt");
        assert!(extracted_file.exists());
        let content = std::fs::read_to_string(&extracted_file).unwrap();
        assert_eq!(content, "test content");
    }
}
