use std::io::Read;
use zip::ZipArchive;
use sha2::{Sha256, Digest};
use lambda_models::LambdaError;
use tracing::{info, instrument};

pub struct ZipHandler {
    max_zip_size: u64,
}

impl ZipHandler {
    pub fn new(max_zip_size: u64) -> Self {
        Self { max_zip_size }
    }

    #[instrument(skip(self, zip_data))]
    pub async fn process_zip(&self, zip_data: &[u8]) -> Result<ZipInfo, LambdaError> {
        // Validate ZIP size
        if zip_data.len() as u64 > self.max_zip_size {
            return Err(LambdaError::CodeTooLarge { 
                size: zip_data.len() as u64, 
                max_size: self.max_zip_size 
            });
        }

        // Compute SHA256 hash
        let mut hasher = Sha256::new();
        hasher.update(zip_data);
        let sha256 = format!("{:x}", hasher.finalize());

        // Validate ZIP structure
        let mut archive = ZipArchive::new(std::io::Cursor::new(zip_data))
            .map_err(|e| LambdaError::InvalidZipFile { reason: e.to_string() })?;

        let mut files = Vec::new();
        let mut total_size = 0;

        for i in 0..archive.len() {
            let file = archive.by_index(i)
                .map_err(|e| LambdaError::InvalidZipFile { reason: e.to_string() })?;
            
            let file_name = file.name().to_string();
            let file_size = file.size();
            
            total_size += file_size;
            files.push(ZipFileInfo {
                name: file_name,
                size: file_size,
                is_executable: file.unix_mode().map_or(false, |mode| mode & 0o111 != 0),
            });
        }

        info!("Processed ZIP with {} files, total size: {} bytes, SHA256: {}", 
              files.len(), total_size, sha256);

        Ok(ZipInfo {
            sha256,
            files,
            total_size,
            zip_data: zip_data.to_vec(),
        })
    }

    #[instrument(skip(self, zip_data))]
    pub async fn extract_to_directory(&self, zip_data: &[u8], target_dir: &std::path::Path) -> Result<(), LambdaError> {
        let mut archive = ZipArchive::new(std::io::Cursor::new(zip_data))
            .map_err(|e| LambdaError::InvalidZipFile { reason: e.to_string() })?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| LambdaError::InvalidZipFile { reason: e.to_string() })?;
            
            let file_path = target_dir.join(file.name());
            
            // Create parent directories if they don't exist
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| LambdaError::InvalidZipFile { reason: e.to_string() })?;
            }
            
            // Skip directories
            if file.name().ends_with('/') {
                std::fs::create_dir_all(&file_path)
                    .map_err(|e| LambdaError::InvalidZipFile { reason: e.to_string() })?;
                continue;
            }
            
            // Extract file
            let mut file_data = Vec::new();
            file.read_to_end(&mut file_data)
                .map_err(|e| LambdaError::InvalidZipFile { reason: e.to_string() })?;
            
            std::fs::write(&file_path, file_data)
                .map_err(|e| LambdaError::InvalidZipFile { reason: e.to_string() })?;
            
            // Set file permissions if available
            if let Some(mode) = file.unix_mode() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let permissions = std::fs::Permissions::from_mode(mode);
                    std::fs::set_permissions(&file_path, permissions)
                        .map_err(|e| LambdaError::InvalidZipFile { reason: e.to_string() })?;
                }
            }
        }

        info!("Extracted ZIP to directory: {}", target_dir.display());
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ZipInfo {
    pub sha256: String,
    pub files: Vec<ZipFileInfo>,
    pub total_size: u64,
    pub zip_data: Vec<u8>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ZipFileInfo {
    pub name: String,
    pub size: u64,
    pub is_executable: bool,
}
