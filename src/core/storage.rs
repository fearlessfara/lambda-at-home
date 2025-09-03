use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::core::models::Function;

pub struct FunctionStorage {
    base_path: PathBuf,
}

impl FunctionStorage {
    pub fn new(base_path: &str) -> Result<Self> {
        let base_path = PathBuf::from(base_path);
        std::fs::create_dir_all(&base_path)
            .context("Failed to create storage directory")?;

        Ok(Self { base_path })
    }

    pub fn get_function_path(&self, function_id: &Uuid) -> PathBuf {
        self.base_path.join(function_id.to_string())
    }

    pub async fn save_function(&self, function: &Function) -> Result<()> {
        let function_path = self.get_function_path(&function.id);
        std::fs::create_dir_all(&function_path)?;

        let metadata_path = function_path.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(function)?;
        std::fs::write(metadata_path, metadata_json)?;

        debug!("Saved function metadata: {}", function.id);
        Ok(())
    }

    pub async fn get_function(&self, function_id: &Uuid) -> Result<Option<Function>> {
        let function_path = self.get_function_path(function_id);
        let metadata_path = function_path.join("metadata.json");

        if !metadata_path.exists() {
            return Ok(None);
        }

        let metadata_json = std::fs::read_to_string(metadata_path)?;
        let function: Function = serde_json::from_str(&metadata_json)?;

        Ok(Some(function))
    }

    pub async fn list_functions(&self) -> Result<Vec<Function>> {
        let mut functions = Vec::new();

        if !self.base_path.exists() {
            return Ok(functions);
        }

        let entries = std::fs::read_dir(&self.base_path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let metadata_path = path.join("metadata.json");
                if metadata_path.exists() {
                    match std::fs::read_to_string(&metadata_path) {
                        Ok(metadata_json) => {
                            match serde_json::from_str::<Function>(&metadata_json) {
                                Ok(function) => functions.push(function),
                                Err(e) => {
                                    warn!("Failed to parse function metadata in {:?}: {}", path, e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read function metadata in {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        // Sort by creation date
        functions.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        Ok(functions)
    }

    pub async fn delete_function(&self, function_id: &Uuid) -> Result<()> {
        let function_path = self.get_function_path(function_id);

        if function_path.exists() {
            std::fs::remove_dir_all(&function_path)?;
            debug!("Deleted function: {}", function_id);
        }

        Ok(())
    }

    pub async fn function_exists(&self, function_id: &Uuid) -> bool {
        let function_path = self.get_function_path(function_id);
        function_path.exists() && function_path.join("metadata.json").exists()
    }

    pub async fn get_function_code_path(&self, function_id: &Uuid) -> PathBuf {
        self.get_function_path(function_id).join("code")
    }

    pub async fn save_function_code(&self, function_id: &Uuid, code: &[u8]) -> Result<()> {
        let code_path = self.get_function_code_path(function_id).await;
        std::fs::create_dir_all(&code_path.parent().unwrap())?;
        std::fs::write(&code_path, code)?;
        Ok(())
    }

    pub async fn get_function_code(&self, function_id: &Uuid) -> Result<Option<Vec<u8>>> {
        let code_path = self.get_function_code_path(function_id).await;
        
        if !code_path.exists() {
            return Ok(None);
        }

        let code = std::fs::read(&code_path)?;
        Ok(Some(code))
    }

    pub async fn cleanup_old_functions(&self, older_than_days: u64) -> Result<()> {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::days(older_than_days as i64);
        let mut functions_to_delete = Vec::new();

        let functions = self.list_functions().await?;
        for function in functions {
            if function.created_at < cutoff_time {
                functions_to_delete.push(function.id);
            }
        }

        for function_id in functions_to_delete {
            info!("Cleaning up old function: {}", function_id);
            self.delete_function(&function_id).await?;
        }

        Ok(())
    }

    pub async fn get_storage_stats(&self) -> Result<StorageStats> {
        let functions = self.list_functions().await?;
        let mut total_size = 0u64;

        for function in &functions {
            let function_path = self.get_function_path(&function.id);
            if function_path.exists() {
                total_size += self.calculate_directory_size(&function_path)?;
            }
        }

        Ok(StorageStats {
            total_functions: functions.len(),
            total_size_bytes: total_size,
            functions_by_status: functions
                .iter()
                .fold(HashMap::new(), |mut acc, f| {
                    let status = format!("{:?}", f.status);
                    *acc.entry(status).or_insert(0) += 1;
                    acc
                }),
        })
    }

    fn calculate_directory_size(&self, path: &Path) -> Result<u64> {
        let mut total_size = 0u64;

        if path.is_file() {
            return Ok(path.metadata()?.len());
        }

        if path.is_dir() {
            let entries = std::fs::read_dir(path)?;
            for entry in entries {
                let entry = entry?;
                total_size += self.calculate_directory_size(&entry.path())?;
            }
        }

        Ok(total_size)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_functions: usize,
    pub total_size_bytes: u64,
    pub functions_by_status: HashMap<String, usize>,
}
