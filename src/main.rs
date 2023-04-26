#![feature(const_trait_impl)]

use serde::Deserialize;
use tokio::process::Command;

mod files;
mod modrinth;

#[derive(Clone, Deserialize)]
pub struct ModPackDescriptor {
    name: String,
    version: String,
    loader: String,
    folder: String,
    target: String,
    fabric: String,
    mods: Vec<String>,
    external: Vec<External>
}

#[derive(Clone, Deserialize)]
pub struct External {
    url: String,
    file: String
}

#[derive(Clone)]
pub struct Target {
    file_path: String,
    target_version: String
}

#[tokio::main]
async fn main() {
    // Check if the '-server' argument was passed.
    let mut server = false;
    for arg in std::env::args() {
        if arg == "-server" {
            server = true;
        }
    }

    // Check if the modpack file exists.
    if !files::exists("modpack.json").await {
        // Check if the running executable is a URL.
        let mut exe_path = std::env::current_exe().unwrap().to_str().unwrap().to_string();
        // Remove the '.exe' and path from the executable name.
        exe_path = exe_path.replace(".exe", "");
        exe_path = exe_path.split("\\").collect::<Vec<&str>>().last().unwrap().to_string();
        exe_path = exe_path.replace("-", "/");
        exe_path = exe_path.replace(";", ":");
        if files::is_url(exe_path.clone()) {
            // Download the modpack file.
            files::download(exe_path, "modpack.json".to_string())
                .await.expect("Unable to download modpack file.");
        } else {
            println!("Modpack file not found.");
            println!("{}", exe_path.clone());
            return;
        }
    }

    // Read the modpack data file.
    let file = files::read("modpack.json").await;
    let decoded = serde_json::from_str::<ModPackDescriptor>(&file).unwrap();
    let modpack = decoded.clone();

    // Check if Minecraft is installed.
    let app_data = files::get_appdata().unwrap();
    let versions_dir = format!("{}/{}/{}",
                               app_data.to_str().unwrap(),
                               ".minecraft", "versions");
    let loader = format!("{}/{}", versions_dir, decoded.loader);
    if !files::exists(&loader).await {
        download_loader(modpack.clone()).await;
    }

    // Check if the modpack is already installed.
    let modpack_dir = format!("{}/{}", versions_dir, decoded.folder);
    if files::exists(&modpack_dir).await {
        // TODO: Update the modpack.
        println!("Modpack already installed.");
        return;
    }

    // Run starting message.
    println!("Installing modpack {} v{}...",
             decoded.name, decoded.version);

    // Create the modpack directory.
    files::create_dir(&modpack_dir).await;
    // Create the 'mods' directory.
    files::create_dir(&format!("{}/{}", modpack_dir.clone(), "mods")).await;
    // Create the 'config' directory.
    files::create_dir(&format!("{}/{}", modpack_dir.clone(), "config")).await;
    // Create the target object.
    let target = Target {
        file_path: modpack_dir.clone(),
        target_version: decoded.target.clone()
    };

    // Download the mods to the target.
    for _mod in decoded.mods {
        modrinth::download(target.clone(), _mod, server).await.unwrap();
    }

    // Download the external mods.
    for external in decoded.external {
        // Check if the file contains a path.
        if external.file.contains("/") {
            // Create the directory.
            let path = format!("{}/{}", modpack_dir.clone(),
                               external.file.split("/").collect::<Vec<&str>>()[0]);
            files::create_dir(&path).await;
        }

        let path = format!("{}/{}", modpack_dir.clone(), external.file);
        files::download(external.url, path).await.unwrap_or_else(|_| {
            println!("Failed to download {}.", external.file);
        });
    }

    // Create a Minecraft profile.
    create_profile(modpack_dir.clone(), modpack).await;

    println!("Modpack installed.");
}

/// Creates a Minecraft profile.
async fn create_profile(modpack_dir: String, modpack: ModPackDescriptor) {
    // Get the .minecraft directory.
    let app_data = files::get_appdata().unwrap();
    let minecraft_dir = format!("{}/{}", app_data.to_str().unwrap(), ".minecraft");

    // Read the JSON file.
    let file = files::read(&format!("{}/{}", minecraft_dir.clone(), "launcher_profiles.json")).await;
    let mut decoded = serde_json::from_str::<serde_json::Value>(&file).unwrap();

    // Get the profiles object.
    let profiles = decoded["profiles"].as_object_mut().unwrap();
    // Create the modpack profile.
    profiles.insert(modpack.name.clone(), serde_json::json!({
        "name": modpack.name.clone(),
        "lastVersionId": modpack.loader.clone(),
        "gameDir": modpack_dir,
        "icon": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAIAAAACABAMAAAAxEHz4AAAAGFBMVEUAAAA4NCrb0LTGvKW8spyAem2uppSakn5SsnMLAAAAAXRSTlMAQObYZgAAAJ5JREFUaIHt1MENgCAMRmFWYAVXcAVXcAVXcH3bhCYNkYjcKO8dSf7v1JASUWdZAlgb0PEmDSMAYYBdGkYApgf8ER3SbwRgesAf0BACMD1gB6S9IbkEEBfwY49oNj4lgLhA64C0o9R9RABTAvp4SX5kB2TA5y8EEAK4pRrxB9QcA4QBWkj3GCAMUCO/xwBhAI/kEsCagCHDY4AwAC3VA6t4zTAMj0OJAAAAAElFTkSuQmCC",
        "javaArgs": "-Xmx4G -XX:+UnlockExperimentalVMOptions -XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M"
    }));

    // Write the JSON file.
    files::write(&format!("{}/{}",
                          minecraft_dir.clone(), "launcher_profiles.json"),
                 decoded.to_string()).await;
}

/// Download and installs the Fabric loader.
async fn download_loader(modpack: ModPackDescriptor) {
    // Get the %tmp% directory.
    let tmp_dir = files::get_temp().unwrap();
    // Download the Fabric Loader to the %tmp% directory.
    files::download("https://maven.fabricmc.net/net/fabricmc/fabric-installer/0.11.2/fabric-installer-0.11.2.jar".to_string(),
                    format!("{}/{}", tmp_dir.to_str().unwrap(), "fabric-installer.jar")).await.unwrap();
    // Run the Fabric Installer.
    Command::new("java")
        .arg("-jar")
        .arg(format!("{}/{}", tmp_dir.to_str().unwrap(), "fabric-installer.jar"))
        .arg("client")
        .arg("-loader")
        .arg(modpack.fabric)
        .arg("-mcversion")
        .arg(modpack.target)
        .spawn().unwrap().wait().await.unwrap();
}