use crate::zip_handler::ZipInfo;
use lambda_models::{Function, LambdaError};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{info, instrument};

pub struct PackagingCache {
    cache_dir: PathBuf,
    image_cache: HashMap<String, String>, // function_id -> image_tag
    zip_cache: HashMap<String, ZipInfo>,  // sha256 -> zip_info
}

impl PackagingCache {
    pub fn new(cache_dir: PathBuf) -> Result<Self, LambdaError> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir).map_err(|e| LambdaError::InternalError {
            reason: e.to_string(),
        })?;

        let mut cache = Self {
            cache_dir,
            image_cache: HashMap::new(),
            zip_cache: HashMap::new(),
        };

        // Load existing cache
        cache.load_cache()?;

        Ok(cache)
    }

    #[instrument(skip(self))]
    pub fn get_cached_image(&self, function: &Function, zip_sha256: &str) -> Option<String> {
        // Include runtime in cache key since different runtimes have different bootstrap scripts
        let cache_key = format!(
            "{}:{}:{}",
            function.function_id, function.runtime, zip_sha256
        );
        self.image_cache.get(&cache_key).cloned()
    }

    #[instrument(skip(self))]
    pub fn cache_image(&mut self, function: &Function, zip_sha256: &str, image_tag: String) {
        // Include runtime in cache key since different runtimes have different bootstrap scripts
        let cache_key = format!(
            "{}:{}:{}",
            function.function_id, function.runtime, zip_sha256
        );
        self.image_cache.insert(cache_key, image_tag);
        info!(
            "Cached image for function: {} with SHA256: {}",
            function.function_name, zip_sha256
        );
    }

    #[instrument(skip(self))]
    pub fn get_cached_zip(&self, sha256: &str) -> Option<&ZipInfo> {
        self.zip_cache.get(sha256)
    }

    #[instrument(skip(self))]
    pub fn cache_zip(&mut self, zip_info: ZipInfo) {
        let sha256 = zip_info.sha256.clone();
        self.zip_cache.insert(sha256.clone(), zip_info);
        info!("Cached ZIP with SHA256: {}", sha256);
    }

    #[instrument(skip(self))]
    pub fn store_zip_file(&self, zip_info: &ZipInfo) -> Result<PathBuf, LambdaError> {
        let zip_path = self.cache_dir.join("zips").join(&zip_info.sha256);

        // Create zip directory if it doesn't exist
        if let Some(parent) = zip_path.parent() {
            fs::create_dir_all(parent).map_err(|e| LambdaError::InternalError {
                reason: e.to_string(),
            })?;
        }

        // Write ZIP file
        fs::write(&zip_path, &zip_info.zip_data).map_err(|e| LambdaError::InternalError {
            reason: e.to_string(),
        })?;

        info!("Stored ZIP file: {}", zip_path.display());
        Ok(zip_path)
    }

    #[instrument(skip(self))]
    pub fn load_zip_file(&self, sha256: &str) -> Result<Vec<u8>, LambdaError> {
        let zip_path = self.cache_dir.join("zips").join(sha256);

        if !zip_path.exists() {
            return Err(LambdaError::InvalidRequest {
                reason: format!("ZIP file not found: {sha256}"),
            });
        }

        let zip_data = fs::read(&zip_path).map_err(|e| LambdaError::InternalError {
            reason: e.to_string(),
        })?;

        Ok(zip_data)
    }

    #[instrument(skip(self))]
    pub fn cleanup_old_cache(&mut self, max_age_days: u64) -> Result<usize, LambdaError> {
        let cutoff_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (max_age_days * 24 * 60 * 60);

        let mut removed_count = 0;

        // Clean up ZIP files
        let zip_dir = self.cache_dir.join("zips");
        if zip_dir.exists() {
            for entry in fs::read_dir(&zip_dir).map_err(|e| LambdaError::InternalError {
                reason: e.to_string(),
            })? {
                let entry = entry.map_err(|e| LambdaError::InternalError {
                    reason: e.to_string(),
                })?;
                let metadata = entry.metadata().map_err(|e| LambdaError::InternalError {
                    reason: e.to_string(),
                })?;

                if let Ok(modified) = metadata.modified() {
                    if let Ok(modified_secs) = modified.duration_since(std::time::UNIX_EPOCH) {
                        if modified_secs.as_secs() < cutoff_time {
                            fs::remove_file(entry.path()).map_err(|e| {
                                LambdaError::InternalError {
                                    reason: e.to_string(),
                                }
                            })?;
                            removed_count += 1;
                        }
                    }
                }
            }
        }

        info!("Cleaned up {} old cache files", removed_count);
        Ok(removed_count)
    }

    fn load_cache(&mut self) -> Result<(), LambdaError> {
        // Load image cache from disk
        let image_cache_file = self.cache_dir.join("image_cache.json");
        if image_cache_file.exists() {
            let cache_data =
                fs::read_to_string(&image_cache_file).map_err(|e| LambdaError::InternalError {
                    reason: e.to_string(),
                })?;

            self.image_cache =
                serde_json::from_str(&cache_data).map_err(|e| LambdaError::InternalError {
                    reason: e.to_string(),
                })?;
        }

        // Load ZIP cache from disk
        let zip_cache_file = self.cache_dir.join("zip_cache.json");
        if zip_cache_file.exists() {
            let cache_data =
                fs::read_to_string(&zip_cache_file).map_err(|e| LambdaError::InternalError {
                    reason: e.to_string(),
                })?;

            self.zip_cache =
                serde_json::from_str(&cache_data).map_err(|e| LambdaError::InternalError {
                    reason: e.to_string(),
                })?;
        }

        Ok(())
    }

    pub fn save_cache(&self) -> Result<(), LambdaError> {
        // Save image cache to disk
        let image_cache_file = self.cache_dir.join("image_cache.json");
        let cache_data = serde_json::to_string_pretty(&self.image_cache).map_err(|e| {
            LambdaError::InternalError {
                reason: e.to_string(),
            }
        })?;
        fs::write(&image_cache_file, cache_data).map_err(|e| LambdaError::InternalError {
            reason: e.to_string(),
        })?;

        // Save ZIP cache to disk
        let zip_cache_file = self.cache_dir.join("zip_cache.json");
        let cache_data = serde_json::to_string_pretty(&self.zip_cache).map_err(|e| {
            LambdaError::InternalError {
                reason: e.to_string(),
            }
        })?;
        fs::write(&zip_cache_file, cache_data).map_err(|e| LambdaError::InternalError {
            reason: e.to_string(),
        })?;

        Ok(())
    }
}
