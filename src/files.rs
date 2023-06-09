use std::env::var_os;
use std::fs::File;
use std::path::PathBuf;
use reqwest::header::USER_AGENT;
use tokio::fs;
use crate::{CLIENT, DEFAULT_AGENT};

/// Checks if the file exists on the file system.
pub async fn exists(path: &str) -> bool {
    fs::metadata(path).await.is_ok()
}

/// Reads the file from the file system.
pub async fn read(path: &str) -> String {
    fs::read_to_string(path).await.unwrap()
}

/// Writes the file to the file system.
pub async fn write(path: &str, content: String) {
    fs::write(path, content).await.unwrap();
}

/// Deletes the file from the file system.
pub async fn delete(path: &str) {
    fs::remove_file(path).await.unwrap();
}

/// Creates a directory on the file system.
pub async fn create_dir(path: &str) {
    // Check if the directory already exists.
    if exists(path).await {
        return;
    }

    fs::create_dir(path).await.unwrap();
}

/// Gets the path to the device's AppData directory.
pub fn get_appdata() -> Option<PathBuf> {
    var_os("APPDATA").map(PathBuf::from)
}

/// Gets the path to the device's temporary directory.
pub fn get_temp() -> Option<PathBuf> {
    var_os("TEMP").map(PathBuf::from)
}

/// Downloads a file from the internet.
/// Saves the file to the file system.
pub async fn download(url: String, path: String) -> Result<(), reqwest::Error> {
    let bytes = CLIENT.get(url).header(USER_AGENT, DEFAULT_AGENT.clone())
        .send().await?.bytes().await?;
    Ok(fs::write(path, bytes).await.unwrap())
}

/// Checks if the URL is valid.
pub fn is_url(url: String) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// Extracts an archive to the file system.
pub fn extract_archive(archive: String, destination: String) {
    let file = File::open(archive).unwrap();
    zip_extract::extract(file, destination.as_ref(), true).unwrap();
}