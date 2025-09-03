use anyhow::Result;
use clap::{Parser, Subcommand};
use reqwest::Client;
use serde_json::json;
use std::path::Path;
use base64::{Engine as _, engine::general_purpose};
use zip::write::FileOptions;

#[derive(Parser)]
#[command(name = "lambda-cli")]
#[command(about = "CLI tool for managing Lambda functions")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    #[arg(long, default_value = "http://localhost:3000")]
    pub server_url: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Deploy a function
    Deploy {
        /// Path to the function directory
        #[arg(short, long)]
        path: String,
        
        /// Function name
        #[arg(short, long)]
        name: String,
        
        /// Function description
        #[arg(short, long)]
        description: Option<String>,
        
        /// Handler file (default: index.js)
        #[arg(long, default_value = "index.js")]
        handler: String,
        
        /// Runtime (default: nodejs18.x)
        #[arg(long, default_value = "nodejs18.x")]
        runtime: String,
        
        /// Memory limit in MB (default: 128)
        #[arg(long, default_value = "128")]
        memory: u64,
        
        /// CPU limit (default: 0.5)
        #[arg(long, default_value = "0.5")]
        cpu: f64,
        
        /// Timeout in seconds (default: 30)
        #[arg(long, default_value = "30")]
        timeout: u64,
    },
    
    /// Invoke a function
    Invoke {
        /// Function ID
        #[arg(short, long)]
        function_id: String,
        
        /// Payload JSON string
        #[arg(short, long)]
        payload: Option<String>,
        
        /// Payload file path
        #[arg(long)]
        payload_file: Option<String>,
        
        /// Timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,
    },
    
    /// List all functions
    List,
    
    /// Get function details
    Get {
        /// Function ID
        function_id: String,
    },
    
    /// Delete a function
    Delete {
        /// Function ID
        function_id: String,
    },
    
    /// Health check
    Health,
}

pub struct LambdaClient {
    client: Client,
    base_url: String,
}

impl LambdaClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn health_check(&self) -> Result<serde_json::Value> {
        let response = self.client
            .get(&format!("{}/", self.base_url))
            .send()
            .await?;
        
        let health: serde_json::Value = response.json().await?;
        Ok(health)
    }

    pub async fn deploy_function(&self, request: serde_json::Value) -> Result<serde_json::Value> {
        let response = self.client
            .post(&format!("{}/functions", self.base_url))
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Deploy failed: {}", error_text));
        }
        
        let function: serde_json::Value = response.json().await?;
        Ok(function)
    }

    pub async fn invoke_function(&self, function_id: &str, payload: serde_json::Value) -> Result<serde_json::Value> {
        let response = self.client
            .post(&format!("{}/functions/{}", self.base_url, function_id))
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Invoke failed: {}", error_text));
        }
        
        let result: serde_json::Value = response.json().await?;
        Ok(result)
    }

    pub async fn list_functions(&self) -> Result<Vec<serde_json::Value>> {
        let response = self.client
            .get(&format!("{}/functions", self.base_url))
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("List failed: {}", error_text));
        }
        
        let functions: Vec<serde_json::Value> = response.json().await?;
        Ok(functions)
    }

    pub async fn get_function(&self, function_id: &str) -> Result<serde_json::Value> {
        let response = self.client
            .get(&format!("{}/functions/{}", self.base_url, function_id))
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Get function failed: {}", error_text));
        }
        
        let function: serde_json::Value = response.json().await?;
        Ok(function)
    }

    pub async fn delete_function(&self, function_id: &str) -> Result<()> {
        let response = self.client
            .delete(&format!("{}/functions/{}", self.base_url, function_id))
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Delete failed: {}", error_text));
        }
        
        Ok(())
    }
}

pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    let client = LambdaClient::new(cli.server_url);

    match cli.command {
        Commands::Health => {
            let health = client.health_check().await?;
            println!("{}", serde_json::to_string_pretty(&health)?);
        }
        
        Commands::Deploy { path, name, description, handler, runtime, memory, cpu, timeout } => {
            let function_path = Path::new(&path);
            if !function_path.exists() {
                return Err(anyhow::anyhow!("Function path does not exist: {}", path));
            }

            // Create zip file from function directory
            let zip_data = create_zip_from_directory(function_path)?;
            let zip_base64 = general_purpose::STANDARD.encode(&zip_data);

            // Read Dockerfile if it exists
            let dockerfile_path = function_path.join("Dockerfile");
            let dockerfile = if dockerfile_path.exists() {
                Some(std::fs::read_to_string(&dockerfile_path)?)
            } else {
                None
            };

            let request = json!({
                "name": name,
                "description": description,
                "handler": handler,
                "runtime": runtime,
                "memory_limit": memory * 1024 * 1024, // Convert MB to bytes
                "cpu_limit": cpu,
                "timeout": timeout,
                "code": {
                    "zip_file": zip_base64,
                    "dockerfile": dockerfile
                }
            });

            let function = client.deploy_function(request).await?;
            println!("Function deployed successfully:");
            println!("{}", serde_json::to_string_pretty(&function)?);
        }
        
        Commands::Invoke { function_id, payload, payload_file, timeout } => {
            let payload_value = if let Some(payload_str) = payload {
                serde_json::from_str(&payload_str)?
            } else if let Some(file_path) = payload_file {
                let file_content = std::fs::read_to_string(&file_path)?;
                serde_json::from_str(&file_content)?
            } else {
                json!({})
            };

            let request = json!({
                "payload": payload_value,
                "timeout": timeout
            });

            let result = client.invoke_function(&function_id, request).await?;
            println!("Function execution result:");
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        
        Commands::List => {
            let functions = client.list_functions().await?;
            println!("Functions:");
            for function in functions {
                println!("{}", serde_json::to_string_pretty(&function)?);
                println!("---");
            }
        }
        
        Commands::Get { function_id } => {
            let function = client.get_function(&function_id).await?;
            println!("Function details:");
            println!("{}", serde_json::to_string_pretty(&function)?);
        }
        
        Commands::Delete { function_id } => {
            client.delete_function(&function_id).await?;
            println!("Function deleted successfully: {}", function_id);
        }
    }

    Ok(())
}

fn create_zip_from_directory(path: &Path) -> Result<Vec<u8>> {
    
    let mut zip_buffer = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);

        add_dir_to_zip(&mut zip, path, "", &options)?;
        zip.finish()?;
    }
    
    Ok(zip_buffer)
}

fn add_dir_to_zip(
    zip: &mut zip::ZipWriter<std::io::Cursor<&mut Vec<u8>>>,
    dir_path: &Path,
    prefix: &str,
    options: &FileOptions,
) -> Result<()> {
    let entries = std::fs::read_dir(dir_path)?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy();
        
        if path.is_file() {
            let file_path = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", prefix, name)
            };
            
            zip.start_file(&file_path, *options)?;
            let mut file = std::fs::File::open(&path)?;
            std::io::copy(&mut file, zip)?;
        } else if path.is_dir() {
            let dir_path = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", prefix, name)
            };
            
            zip.add_directory(&dir_path, *options)?;
            add_dir_to_zip(zip, &path, &dir_path, options)?;
        }
    }
    
    Ok(())
}
