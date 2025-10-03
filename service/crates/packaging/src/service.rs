use crate::{ImageBuilder, PackagingCache, ZipHandler};
use lambda_models::{Config, Function, LambdaError};
use std::path::PathBuf;

pub struct PackagingService {
    zip_handler: ZipHandler,
    image_builder: ImageBuilder,
    cache: PackagingCache,
}

impl PackagingService {
    pub fn new(config: Config) -> Self {
        let zip_handler = ZipHandler::new(50 * 1024 * 1024); // 50MB limit
        let image_builder = ImageBuilder::new(config.docker.host.clone());
        let cache = PackagingCache::new(config.data.dir.clone().into()).unwrap_or_else(|_| {
            // Create a default cache if the directory doesn't exist
            PackagingCache::new(PathBuf::from("./data")).unwrap()
        });

        Self {
            zip_handler,
            image_builder,
            cache,
        }
    }

    pub async fn process_zip(&self, zip_data: &[u8]) -> Result<crate::ZipInfo, LambdaError> {
        self.zip_handler.process_zip(zip_data).await
    }

    pub async fn build_image(
        &mut self,
        function: &Function,
        image_ref: &str,
        runtime_api_port: u16,
    ) -> Result<(), LambdaError> {
        // Get the ZIP data for this function
        let zip_data = self.cache.load_zip_file(&function.code_sha256)?;
        let zip_info = self.zip_handler.process_zip(&zip_data).await?;

        // Check cache first
        if let Some(_cached_image) = self.cache.get_cached_image(function, &zip_info.sha256) {
            return Ok(());
        }

        // Build new image
        self.image_builder
            .build_image(function, &zip_info, image_ref, runtime_api_port)
            .await?;

        // Cache the result
        self.cache
            .cache_image(function, &zip_info.sha256, image_ref.to_string());

        Ok(())
    }

    pub fn store_zip(&self, zip_info: &crate::ZipInfo) -> Result<PathBuf, LambdaError> {
        self.cache.store_zip_file(zip_info)
    }

    pub fn load_zip(&self, sha256: &str) -> Result<Vec<u8>, LambdaError> {
        self.cache.load_zip_file(sha256)
    }

    pub fn save_cache(&self) -> Result<(), LambdaError> {
        self.cache.save_cache()
    }
}
