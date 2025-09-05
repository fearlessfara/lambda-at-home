#![cfg(feature = "docker_tests")]

use lambda_testsupport::*;
use std::io::Write;
use std::time::Duration;

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn function_error_handled() -> anyhow::Result<()> {
    let mut daemon = spawn_daemon(None).await?;

    // Create a function that throws an error
    let error_js = r#"
exports.handler = async (event) => {
    console.log("About to throw error");
    throw new Error("This is a handled error");
};
"#;

    let mut zip_data = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_data));
        zip.start_file("index.js", zip::write::FileOptions::default())?;
        zip.write_all(error_js.as_bytes())?;
        zip.finish()?;
    }

    let name = "error-function";
    create_function(
        &daemon,
        serde_json::json!({
            "FunctionName": name,
            "Runtime": "nodejs18.x",
            "Handler": "index.handler",
            "MemorySize": 256,
            "Timeout": 5,
            "Code": { "ZipFile": b64(&zip_data) }
        }),
    )
    .await?;

    let resp = invoke(
        &daemon,
        name,
        serde_json::json!({"test":"error"}),
        Some(("RequestResponse", "Tail")),
    )
    .await?;

    // Should return 200 with error header
    assert_eq!(resp.status_code, 200);
    assert!(resp.function_error.is_some());
    assert_eq!(
        resp.function_error,
        Some(lambda_models::FunctionError::Unhandled)
    );

    // Should have error message in payload
    let payload = as_json(&resp.payload)?;
    assert!(payload.get("errorMessage").is_some());
    assert!(payload.get("errorType").is_some());

    daemon.kill().await?;
    Ok(())
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn function_timeout() -> anyhow::Result<()> {
    let mut daemon = spawn_daemon(None).await?;

    // Create a function that sleeps longer than timeout
    let timeout_js = r#"
exports.handler = async (event) => {
    console.log("Starting long sleep");
    await new Promise(resolve => setTimeout(resolve, 10000)); // 10 seconds
    return { message: "This should not be reached" };
};
"#;

    let mut zip_data = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_data));
        zip.start_file("index.js", zip::write::FileOptions::default())?;
        zip.write_all(timeout_js.as_bytes())?;
        zip.finish()?;
    }

    let name = "timeout-function";
    create_function(
        &daemon,
        serde_json::json!({
            "FunctionName": name,
            "Runtime": "nodejs18.x",
            "Handler": "index.handler",
            "MemorySize": 256,
            "Timeout": 2, // 2 second timeout
            "Code": { "ZipFile": b64(&zip_data) }
        }),
    )
    .await?;

    let resp = invoke(
        &daemon,
        name,
        serde_json::json!({"test":"timeout"}),
        Some(("RequestResponse", "Tail")),
    )
    .await?;

    // Should return 200 with timeout error
    assert_eq!(resp.status_code, 200);
    assert!(resp.function_error.is_some());
    assert_eq!(
        resp.function_error,
        Some(lambda_models::FunctionError::Unhandled)
    );

    // Should have timeout error message
    let payload = as_json(&resp.payload)?;
    assert!(payload.get("errorMessage").is_some());
    let error_message = payload.get("errorMessage").unwrap().as_str().unwrap();
    assert!(error_message.contains("timeout") || error_message.contains("Timeout"));

    daemon.kill().await?;
    Ok(())
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn invalid_function_name() -> anyhow::Result<()> {
    let mut daemon = spawn_daemon(None).await?;

    let client = lambda_testsupport::LambdaClient::new(daemon.user_api_url.clone());

    // Try to create function with invalid name
    let result = client
        .create_function(lambda_models::CreateFunctionRequest {
            function_name: "invalid-name-with-special-chars!@#".to_string(),
            runtime: "nodejs18.x".to_string(),
            role: None,
            handler: "index.handler".to_string(),
            code: lambda_models::FunctionCode {
                zip_file: Some("dGVzdA==".to_string()), // "test" in base64
                s3_bucket: None,
                s3_key: None,
                s3_object_version: None,
            },
            description: None,
            timeout: Some(5),
            memory_size: Some(256),
            environment: None,
            publish: None,
        })
        .await;

    // Should fail with 400 error
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("InvalidParameterValueException")
            || error.to_string().contains("invalid")
    );

    daemon.kill().await?;
    Ok(())
}
