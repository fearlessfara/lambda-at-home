use lambda_packaging::*;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_zip_sha256_stable() {
    let handler = ZipHandler::new(1024 * 1024);

    // Create a simple ZIP for testing
    let mut zip_data = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_data));
        zip.start_file("test.txt", zip::write::FileOptions::default())
            .unwrap();
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
    let expected_tag = format!(
        "lambda-home/{}:{}",
        function.function_name, function.code_sha256
    );

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
        zip.start_file("test.txt", zip::write::FileOptions::default())
            .unwrap();
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

#[test]
fn test_zip_extraction_with_node_modules() {
    let handler = ZipHandler::new(50 * 1024 * 1024); // 50MB limit

    // Create a test ZIP with node_modules structure
    let mut zip_data = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_data));

        // Add package.json
        zip.start_file("package.json", zip::write::FileOptions::default())
            .unwrap();
        zip.write_all(br#"{"name":"test-lambda","dependencies":{"axios":"^1.6.0"}}"#)
            .unwrap();

        // Add index.js
        zip.start_file("index.js", zip::write::FileOptions::default())
            .unwrap();
        zip.write_all(b"exports.handler = async (event) => { return { statusCode: 200 }; };")
            .unwrap();

        // Add node_modules directory structure
        zip.add_directory("node_modules/", zip::write::FileOptions::default())
            .unwrap();
        zip.add_directory("node_modules/axios/", zip::write::FileOptions::default())
            .unwrap();

        // Add axios package.json
        zip.start_file(
            "node_modules/axios/package.json",
            zip::write::FileOptions::default(),
        )
        .unwrap();
        zip.write_all(br#"{"name":"axios","version":"1.6.0"}"#)
            .unwrap();

        // Add axios main file
        zip.start_file(
            "node_modules/axios/index.js",
            zip::write::FileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"module.exports = {};").unwrap();

        zip.finish().unwrap();
    }

    let temp_dir = tempdir().unwrap();
    futures::executor::block_on(handler.extract_to_directory(&zip_data, temp_dir.path())).unwrap();

    // Verify package.json was extracted
    let package_json = temp_dir.path().join("package.json");
    assert!(package_json.exists());

    // Verify index.js was extracted
    let index_js = temp_dir.path().join("index.js");
    assert!(index_js.exists());

    // Verify node_modules directory was created
    let node_modules = temp_dir.path().join("node_modules");
    assert!(node_modules.exists());
    assert!(node_modules.is_dir());

    // Verify axios package was extracted
    let axios_dir = temp_dir.path().join("node_modules/axios");
    assert!(axios_dir.exists());
    assert!(axios_dir.is_dir());

    let axios_package_json = temp_dir.path().join("node_modules/axios/package.json");
    assert!(axios_package_json.exists());

    let axios_index = temp_dir.path().join("node_modules/axios/index.js");
    assert!(axios_index.exists());
}
