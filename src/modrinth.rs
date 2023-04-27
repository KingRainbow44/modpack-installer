use async_recursion::async_recursion;
use reqwest::header::USER_AGENT;
use serde::{Deserialize};
use tokio::fs;
use tokio::time::Duration;

use crate::{CLIENT, DEFAULT_AGENT, files, Target};

const MODRINTH_API: &str = "https://api.modrinth.com/v2";

#[derive(Clone, Deserialize)]
pub struct ModrinthModInfo {
    id: String,
    title: String,
    client_side: String,
    server_side: String,
    versions: Vec<String>
}

#[derive(Clone, Deserialize)]
pub struct ModrinthModVersion {
    project_id: Option<String>,
    files: Vec<ModrinthFile>,
    dependencies: Vec<ModrinthDependency>,
    game_versions: Vec<String>,
    loaders: Vec<String>
}

#[derive(Clone, Deserialize)]
pub struct ModrinthFile {
    url: String,
    filename: String
}

#[derive(Clone, Deserialize)]
pub struct ModrinthDependency {
    project_id: String,
    dependency_type: String
}

/// Constructs a URL for the Modrinth API.
/// Specifically targets the information about a mod.
fn mod_info(project: String) -> String {
    format!("{}/project/{}", MODRINTH_API, project)
}

/// Constructs a URL for the Modrinth API.
/// Specifically targets the information about a mod's version.
fn version_info(_mod: ModrinthModInfo, version: String) -> String {
    format!("{}/project/{}/version/{}", MODRINTH_API, _mod.id, version)
}

/// Performs a request to the Modrinth API.
/// Handles the rate limit system implemented.
#[async_recursion]
async fn make_request(url: String) -> Result<String, reqwest::Error> {
    let response = CLIENT.get(url.clone())
        .header(USER_AGENT, DEFAULT_AGENT.clone())
        .send().await?;

    // Check if the request was successful.
    if response.status().eq(&429) {
        // Get the 'X-Ratelimit-Reset' header.
        let reset = response.headers().get("X-Ratelimit-Reset")
            .unwrap().to_str().unwrap().parse::<u64>().unwrap() + 1;

        println!("Hit a rate limit; waiting {}s...", reset);

        // Wait 1 minute before making requests again.
        tokio::time::sleep(Duration::from_secs(reset)).await;
        println!("Retrying {}...", url);

        return make_request(url).await;
    }

    // Return the response.
    Ok(response.text().await.unwrap())
}

/// Picks the correct version from the mod's versions.
async fn pick_version(game_ver: String, mut _mod: ModrinthModInfo) -> ModrinthModVersion {
    let mut version = ModrinthModVersion {
        files: vec![], dependencies: vec![],
        game_versions: vec![], loaders: vec![],
        project_id: None
    };

    // Get the mod's versions.
    let mut versions = _mod.clone().versions;
    versions.reverse(); // Reversing increases the chance of finding a compatible version.

    // Iterate through the game versions.
    for (_, ver) in versions.iter().enumerate() {
        // Query the version data.
        version = serde_json::from_str(make_request(
            version_info(_mod.clone(), ver.clone())
        ).await.unwrap().as_str()).unwrap_or_else(|error| {
            println!("Unable to download {} ({}). Error: {}", _mod.clone().title, _mod.clone().id, error);
            ModrinthModVersion {
                files: vec![], dependencies: vec![],
                game_versions: vec![], loaders: vec![],
                project_id: None
            }
        });

        // Check if the version is compatible.
        if version.game_versions.contains(&game_ver) &&
            version.loaders.contains(&"fabric".to_string()) {
            break;
        }
    }

    version
}

/// Saves the mod's version to the file system.
async fn save_version(target: Target, version: ModrinthModVersion, _mod: ModrinthModInfo) -> Result<(), reqwest::Error> {
    // Check if the mod doesn't exist.
    if version.files.len() < 1 {
        println!("Skipped {} ({}).", _mod.title, version.project_id.unwrap_or("".to_string()));
        return Ok(());
    }

    // Get the URL & file name for the mod.
    let url = &version.files[0].url;
    let file_name = &version.files[0].filename;
    // URL decode the file name.
    let file_name = percent_encoding::percent_decode_str(file_name)
        .decode_utf8().unwrap();
    let path = format!("{}/mods/{}", target.clone().file_path, file_name);

    // Check if the file already exists.
    if files::exists(path.clone().as_str()).await {
        return Ok(());
    }

    // Download the mod.
    let bytes = CLIENT.get(url)
        .header(USER_AGENT, DEFAULT_AGENT.clone())
        .send().await?.bytes().await?;
    // Save the mod to the target destination.
    fs::write(path, bytes).await
        .expect("Failed to save mod.");

    println!("Downloaded {} ({}).", _mod.title, version.project_id.unwrap_or("".to_string()));
    Ok(())
}

/// Attempts to download a mod from Modrinth.
/// No checks are performed.
async fn download_unsafe(target: Target, _mod: String) -> Result<(), reqwest::Error> {
    // Get the mod's info.
    let mod_info: ModrinthModInfo = serde_json::from_str(make_request(
        mod_info(_mod.clone())
    ).await.unwrap().as_str()).unwrap();

    // Get the matching version.
    let version_info = pick_version(
        target.clone().target_version, mod_info.clone()).await;

    // Save the version to the file system.
    Ok(save_version(target.clone(), version_info, mod_info).await?)
}

/// Downloads a mod from Modrinth.
/// Checks for dependencies.
pub async fn download(target: Target, _mod: String, is_server: bool) -> Result<bool, reqwest::Error> {
    // Get the mod's info.
    let mod_info: ModrinthModInfo = serde_json::from_str(make_request(
        mod_info(_mod.clone())
    ).await.unwrap().as_str()).unwrap();

    // Get the matching version.
    let version_info = pick_version(
        target.clone().target_version, mod_info.clone()).await;

    // Check if the mod is supported.
    if is_server {
        if mod_info.server_side == "unsupported" {
            return Ok(false);
        }
    } else {
        if mod_info.client_side == "unsupported" {
            return Ok(false);
        }
    }

    // Check if other mods are required.
    let dependencies = version_info.clone().dependencies;
    if dependencies.len() > 0 {
        // Iterate through the dependencies.
        for dependency in dependencies {
            // Check if the dependency is a mod.
            if dependency.dependency_type == "required_mod" {
                download_unsafe(target.clone(), dependency.project_id).await?;
            }
        }
    }

    // Save the version to the file system.
    Ok(save_version(target.clone(), version_info, mod_info).await.is_ok())
}