use serde::{Deserialize};
use tokio::fs;

use crate::{files, Target};

const MODRINTH_API: &str = "https://api.modrinth.com/v2";

#[derive(Clone, Deserialize)]
pub struct ModrinthModInfo {
    id: String,
    client_side: String,
    server_side: String,
    game_versions: Vec<String>,
    versions: Vec<String>
}

#[derive(Clone, Deserialize)]
pub struct ModrinthModVersion {
    project_id: String,
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

/// Picks the correct version from the mod's versions.
async fn pick_version(game_ver: String, _mod: ModrinthModInfo) -> ModrinthModVersion {
    let mut version = ModrinthModVersion {
        files: vec![], dependencies: vec![],
        game_versions: vec![], loaders: vec![]
    };

    // Iterate through the game versions.
    for (i, v) in _mod.versions.iter().enumerate() {
        // Query the version data.
        version = serde_json::from_str(reqwest::get(
            version_info(_mod.clone(), v.clone())
        ).await.unwrap().text().await.unwrap().as_str()).unwrap();

        // Check if the version is compatible.
        if version.game_versions.contains(&game_ver) &&
            version.loaders.contains(&"fabric".to_string()) {
            break;
        }
    }

    version
}

/// Saves the mod's version to the file system.
async fn save_version(target: Target, version: ModrinthModVersion) -> Result<(), reqwest::Error> {
    // Check if the mod doesn't exist.
    if version.files.len() < 1 {
        println!("Skipped {}.", version.project_id);
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
    let bytes = reqwest::get(url).await?.bytes().await?;
    // Save the mod to the target destination.
    fs::write(path, bytes).await
        .expect("Failed to save mod.");

    Ok(())
}

/// Attempts to download a mod from Modrinth.
/// No checks are performed.
async fn download_unsafe(target: Target, _mod: String) -> Result<(), reqwest::Error> {
    // Get the mod's info.
    let mod_info: ModrinthModInfo = serde_json::from_str(reqwest::get(
        mod_info(_mod.clone())
    ).await?.text().await?.as_str()).unwrap();

    // Get the matching version.
    let version_info = pick_version(
        target.clone().target_version, mod_info.clone()).await;

    // Save the version to the file system.
    Ok(save_version(target.clone(), version_info).await?)
}

/// Downloads a mod from Modrinth.
/// Checks for dependencies.
pub async fn download(target: Target, _mod: String, is_server: bool) -> Result<bool, reqwest::Error> {
    // Get the mod's info.
    let mod_info: ModrinthModInfo = serde_json::from_str(reqwest::get(
        mod_info(_mod.clone())
    ).await?.text().await?.as_str()).unwrap();

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
    Ok(save_version(target.clone(), version_info).await.is_ok())
}