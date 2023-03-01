use crate::error::CliError;
use async_trait::async_trait;
use directories::ProjectDirs;
use serde_json;
use std::{
    env::current_dir,
    fs::{self, create_dir_all, File},
    io::copy,
    path::PathBuf,
};

#[async_trait]
pub trait Resolve {
    async fn resolve(&self, path: &str) -> Result<Vec<u8>, CliError>;
}

pub struct PackageManager;

#[async_trait]
impl Resolve for PackageManager {
    async fn resolve(&self, path: &str) -> Result<Vec<u8>, CliError> {
        let splitter = path.split_once(":");

        let Some((specifier, package_path)) = splitter else {
           return fetch_local(path);
        };

        match specifier {
            "https" => return fetch_url("https://www.", package_path).await,
            "pkgs" => return fetch_registry(package_path).await,
            other => return Err(CliError::Specifier(other.to_string())),
        }
    }
}

async fn fetch_url(url_root: &str, package_path: &str) -> Result<Vec<u8>, CliError> {
    let mut cache_path = match ProjectDirs::from("org", "modmark", "packages") {
        Some(path) => path.cache_dir().to_path_buf(),
        None => return Err(CliError::Cache),
    };

    cache_path.push(PathBuf::from(&package_path));

    let mut path = cache_path.clone();
    path.pop();

    create_dir_all(PathBuf::from(&path))?;

    if cache_path.exists() {
        Ok(fs::read(cache_path)?)
    } else {
        let mut website = url_root.to_string();
        website.push_str(&package_path);

        let response = reqwest::get(website).await?;
        let content = response.text().await?;

        let mut file = File::create(&cache_path)?;

        copy(&mut content.as_bytes(), &mut file)?;

        Ok(fs::read(cache_path)?)
    }
}

async fn fetch_registry(package_name: &str) -> Result<Vec<u8>, CliError> {
    let mut cache_path = match ProjectDirs::from("org", "modmark", "packages") {
        Some(path) => path.cache_dir().to_path_buf(),
        None => return Err(CliError::Cache),
    };
    let mut file_name = package_name.to_string();
    file_name.push_str(".wasm");

    cache_path.push("pkgs");
    cache_path.push(&file_name);

    if cache_path.exists() {
        Ok(fs::read(cache_path)?)
    } else {
        cache_path.pop();
        create_dir_all(PathBuf::from(&cache_path))?;
        cache_path.push(&file_name);

        let registry_link = "https://raw.githubusercontent.com/modmark-org/package-registry/main/package-registry.json";
        let registry = reqwest::get(registry_link).await?;
        let registry_content = registry.text().await?;

        let json_content: serde_json::Value = serde_json::from_str(&registry_content)?;

        let package_link = &json_content[&package_name]["source"];
        let Some(package_link) = package_link.as_str() else {return Err(CliError::Registry)};
        let package_response = reqwest::get(package_link).await?;

        let package_content = package_response.text().await?;

        let mut file = File::create(&cache_path)?;

        copy(&mut package_content.as_bytes(), &mut file)?;
        Ok(fs::read(cache_path)?)
    }
}

fn fetch_local(package_path: &str) -> Result<Vec<u8>, CliError> {
    let mut package_path = package_path.to_string();
    package_path.push_str(".wasm");

    let mut local_path = current_dir()?;
    local_path.push(PathBuf::from(&package_path));

    if !local_path.exists() {
        return Err(CliError::Local(package_path));
    }

    return Ok(fs::read(local_path)?);
}
