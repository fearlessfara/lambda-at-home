use clap::{Parser, Subcommand};
use lambda_models::{CreateFunctionRequest, FunctionCode};
use reqwest::Client;
use serde_json::json;
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "lambda-cli")]
#[command(about = "CLI tool for Lambda@Home")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "http://localhost:9000")]
    endpoint: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new function
    Create {
        /// Function name
        name: String,
        /// Runtime (nodejs18.x, python3.11, rust)
        runtime: String,
        /// Handler
        handler: String,
        /// ZIP file path
        zip_file: PathBuf,
        /// Function description
        #[arg(long)]
        description: Option<String>,
        /// Memory size in MB
        #[arg(long, default_value = "512")]
        memory: u64,
        /// Timeout in seconds
        #[arg(long, default_value = "3")]
        timeout: u64,
    },
    /// List functions
    List,
    /// Get function details
    Get {
        /// Function name
        name: String,
    },
    /// Delete a function
    Delete {
        /// Function name
        name: String,
    },
    /// Invoke a function
    Invoke {
        /// Function name
        name: String,
        /// Payload (JSON string)
        #[arg(long)]
        payload: Option<String>,
        /// Invocation type
        #[arg(long, default_value = "RequestResponse")]
        invocation_type: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let client = Client::new();

    match cli.command {
        Commands::Create {
            name,
            runtime,
            handler,
            zip_file,
            description,
            memory,
            timeout,
        } => {
            create_function(
                &client,
                &cli.endpoint,
                CreateFunctionParams {
                    name,
                    runtime,
                    handler,
                    zip_file,
                    description,
                    memory,
                    timeout,
                },
            )
            .await?;
        }
        Commands::List => {
            list_functions(&client, &cli.endpoint).await?;
        }
        Commands::Get { name } => {
            get_function(&client, &cli.endpoint, name).await?;
        }
        Commands::Delete { name } => {
            delete_function(&client, &cli.endpoint, name).await?;
        }
        Commands::Invoke {
            name,
            payload,
            invocation_type,
        } => {
            invoke_function(&client, &cli.endpoint, name, payload, invocation_type).await?;
        }
    }

    Ok(())
}

#[derive(Debug)]
struct CreateFunctionParams {
    name: String,
    runtime: String,
    handler: String,
    zip_file: PathBuf,
    description: Option<String>,
    memory: u64,
    timeout: u64,
}

async fn create_function(
    client: &Client,
    endpoint: &str,
    params: CreateFunctionParams,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Creating function: {}", params.name);

    // Read ZIP file
    let zip_data = std::fs::read(&params.zip_file)?;
    let zip_base64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &zip_data);

    let request = CreateFunctionRequest {
        function_name: params.name.clone(),
        runtime: params.runtime,
        role: None,
        handler: params.handler,
        code: FunctionCode {
            zip_file: Some(zip_base64),
            s3_bucket: None,
            s3_key: None,
            s3_object_version: None,
        },
        description: params.description,
        timeout: Some(params.timeout * 1000), // Convert to milliseconds
        memory_size: Some(params.memory),
        environment: None,
        publish: Some(false),
    };

    let response = client
        .post(format!("{endpoint}/2015-03-31/functions"))
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        let function: lambda_models::Function = response.json().await?;
        println!("✅ Function created successfully:");
        println!("   Name: {}", function.function_name);
        println!("   Runtime: {}", function.runtime);
        println!("   Handler: {}", function.handler);
        println!("   Memory: {} MB", function.memory_size);
        println!("   Timeout: {} ms", function.timeout);
    } else {
        let error_text = response.text().await?;
        error!("Failed to create function: {}", error_text);
        return Err(error_text.into());
    }

    Ok(())
}

async fn list_functions(client: &Client, endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Listing functions");

    let response = client
        .get(format!("{endpoint}/2015-03-31/functions"))
        .send()
        .await?;

    if response.status().is_success() {
        let list_response: lambda_models::ListFunctionsResponse = response.json().await?;
        println!("📋 Functions:");
        for function in list_response.functions {
            println!(
                "   • {} ({}) - {} MB, {} ms",
                function.function_name, function.runtime, function.memory_size, function.timeout
            );
        }
    } else {
        let error_text = response.text().await?;
        error!("Failed to list functions: {}", error_text);
        return Err(error_text.into());
    }

    Ok(())
}

async fn get_function(
    client: &Client,
    endpoint: &str,
    name: String,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Getting function: {}", name);

    let response = client
        .get(format!("{endpoint}/2015-03-31/functions/{name}"))
        .send()
        .await?;

    if response.status().is_success() {
        let function: lambda_models::Function = response.json().await?;
        println!("📋 Function details:");
        println!("   Name: {}", function.function_name);
        println!("   Runtime: {}", function.runtime);
        println!("   Handler: {}", function.handler);
        println!("   Memory: {} MB", function.memory_size);
        println!("   Timeout: {} ms", function.timeout);
        println!("   State: {:?}", function.state);
        println!("   Last Modified: {}", function.last_modified);
        if let Some(desc) = function.description {
            println!("   Description: {desc}");
        }
    } else {
        let error_text = response.text().await?;
        error!("Failed to get function: {}", error_text);
        return Err(error_text.into());
    }

    Ok(())
}

async fn delete_function(
    client: &Client,
    endpoint: &str,
    name: String,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Deleting function: {}", name);

    let response = client
        .delete(format!("{endpoint}/2015-03-31/functions/{name}"))
        .send()
        .await?;

    if response.status().is_success() {
        println!("✅ Function deleted successfully: {name}");
    } else {
        let error_text = response.text().await?;
        error!("Failed to delete function: {}", error_text);
        return Err(error_text.into());
    }

    Ok(())
}

async fn invoke_function(
    client: &Client,
    endpoint: &str,
    name: String,
    payload: Option<String>,
    invocation_type: String,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Invoking function: {}", name);

    let payload_value = if let Some(payload_str) = payload {
        serde_json::from_str(&payload_str)?
    } else {
        json!({"test": "payload"})
    };

    let request = client
        .post(format!(
            "{endpoint}/2015-03-31/functions/{name}/invocations"
        ))
        .header("X-Amz-Invocation-Type", invocation_type)
        .json(&payload_value);

    let response = request.send().await?;

    println!("📤 Invocation response:");
    println!("   Status: {}", response.status());

    // Print headers
    for (key, value) in response.headers() {
        if key.as_str().starts_with("x-amz-") {
            println!("   {}: {}", key, value.to_str().unwrap_or(""));
        }
    }

    let response_text = response.text().await?;
    println!("   Body: {response_text}");

    Ok(())
}
