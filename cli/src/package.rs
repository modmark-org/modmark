use crate::error::CliError;
use directories::ProjectDirs;
use futures::future::join_all;
use modmark_core::Resolve;
use serde_json;
use std::{
    env::current_dir,
    fs::{self, create_dir_all, File},
    io::copy,
    path::PathBuf,
};

static RUNTIME: once_cell::sync::Lazy<tokio::runtime::Runtime> = once_cell::sync::Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
});

pub struct PackageManager {}

impl Resolve for PackageManager {
    type Error = CliError;
    fn resolve(&self, path: &str) -> Result<Vec<u8>, Self::Error> {
        RUNTIME.block_on(resolve_package(path))
    }
    fn resolve_all(&self, paths: Vec<&str>) -> Vec<Result<Vec<u8>, Self::Error>> {
        RUNTIME.block_on(resolve_packages(paths))
    }
}

async fn resolve_packages(paths: Vec<&str>) -> Vec<Result<Vec<u8>, CliError>> {
    let futures = paths.iter().map(|path| resolve_package(path));

    return join_all(futures).await;
}

async fn resolve_package(path: &str) -> Result<Vec<u8>, CliError> {
    let splitter = path.split_once(':');

    let Some((specifier, package_path)) = splitter else {
           return fetch_local(path);
        };

    match specifier {
        "http" => fetch_url(&path).await,
        "https" => fetch_url(&path).await,
        "pkgs" => fetch_registry(package_path).await,
        other => Err(CliError::Specifier(other.to_string())),
    }
}

async fn fetch_url(package_path: &str) -> Result<Vec<u8>, CliError> {
    let mut cache_path = match ProjectDirs::from("org", "modmark", "packages") {
        Some(path) => path.cache_dir().to_path_buf(),
        None => return Err(CliError::Cache),
    };

    let splitter = package_path.split_once(':');
    let Some((_, mut domain_path)) = splitter else {
        return Err(CliError::Cache)
    };

    if domain_path.len() < 2 {
        return Err(CliError::Cache);
    }

    domain_path = &domain_path[2..];

    cache_path.push(PathBuf::from(&domain_path));

    let Some(path) = cache_path.parent() else {return Err(CliError::Cache)};

    create_dir_all(path)?;

    if cache_path.exists() {
        Ok(fs::read(cache_path)?)
    } else {
        let response = reqwest::get(package_path).await?;

        if response.status() != 200 {
            return Err(CliError::Get(response.status().to_string()));
        }

        let content = response.bytes().await?;

        let mut file = File::create(&cache_path)?;

        copy(&mut content.as_ref(), &mut file)?;

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

        if package_response.status() != 200 {
            return Err(CliError::Get(package_response.status().to_string()));
        }

        let package_content = package_response.bytes().await?;

        let mut file = File::create(&cache_path)?;

        copy(&mut package_content.as_ref(), &mut file)?;

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
