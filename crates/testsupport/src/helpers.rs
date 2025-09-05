use anyhow::Result;
use base64::Engine;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use zip::ZipWriter;

/// Base64 encode bytes
pub fn b64<T: AsRef<[u8]>>(bytes: T) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Create a ZIP file from a directory
pub fn zip_dir(path: &Path) -> Result<Vec<u8>> {
    let mut zip_data = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_data));
        add_dir_to_zip(&mut zip, path, "")?;
        zip.finish()?;
    }
    Ok(zip_data)
}

fn add_dir_to_zip(
    zip: &mut ZipWriter<std::io::Cursor<&mut Vec<u8>>>,
    dir_path: &Path,
    zip_path: &str,
) -> Result<()> {
    for entry in std::fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        let new_zip_path = if zip_path.is_empty() {
            name_str.to_string()
        } else {
            format!("{}/{}", zip_path, name_str)
        };

        if path.is_dir() {
            add_dir_to_zip(zip, &path, &new_zip_path)?;
        } else {
            let content = std::fs::read(&path)?;
            zip.start_file(&new_zip_path, zip::write::FileOptions::default())?;
            zip.write_all(&content)?;
        }
    }
    Ok(())
}

/// Poll until a condition is met or timeout
pub async fn poll_until<F, Fut>(
    description: &str,
    timeout_duration: Duration,
    mut condition: F,
) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<bool>>,
{
    let start = std::time::Instant::now();
    let poll_interval = Duration::from_millis(100);

    loop {
        if start.elapsed() >= timeout_duration {
            anyhow::bail!("Timeout waiting for: {}", description);
        }

        if condition().await? {
            return Ok(());
        }

        tokio::time::sleep(poll_interval).await;
    }
}

/// Get example function path
pub fn example_path(name: &str) -> PathBuf {
    std::env::current_dir().unwrap().join("examples").join(name)
}

/// Convert response payload to JSON
pub fn as_json(payload: &Option<serde_json::Value>) -> Result<serde_json::Value> {
    match payload {
        Some(value) => Ok(value.clone()),
        None => Ok(serde_json::Value::Null),
    }
}
