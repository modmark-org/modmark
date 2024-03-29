use std::{
    env::current_dir,
    fs::{self, create_dir_all, File},
    io::copy,
    path::PathBuf,
};

use directories::ProjectDirs;
use futures::future::join_all;
use tokio::sync::mpsc::Sender;

use modmark_core::package_store::{PackageID, PackageSource, Resolve, ResolveTask};

use crate::error::CliError;

#[derive(Clone)]
pub struct PackageManager {
    pub(crate) catalog: String,
    pub(crate) complete_tx: Sender<()>,
}

impl Resolve for PackageManager {
    fn resolve_all(&self, paths: Vec<ResolveTask>) {
        // I don't know how to get rid of all these clones...
        let self_clone = self.clone();
        tokio::spawn(async move {
            join_all(
                paths
                    .into_iter()
                    .map(|task| {
                        let another_self = self_clone.clone();
                        tokio::spawn(async move { another_self.resolve(task).await })
                    })
                    .collect::<Vec<_>>(),
            )
            .await;
            self_clone.complete_tx.send(()).await.unwrap();
        });
    }
}

pub(crate) fn cache_location() -> Result<PathBuf, CliError> {
    match ProjectDirs::from("org", "modmark", "packages") {
        Some(path) => return Ok(path.cache_dir().to_path_buf()),
        None => return Err(CliError::Cache),
    };
}

impl PackageManager {
    async fn resolve(&self, task: ResolveTask) {
        let PackageID {
            name,
            source: target,
        } = &task.package_id;
        let result = match target {
            PackageSource::Local => self.fetch_local(name),
            PackageSource::Catalog => self.fetch_catalog(name).await,
            PackageSource::Url => self.fetch_url(name).await,
            PackageSource::Standard => Err(CliError::Catalog),
        };
        task.complete(result);
    }

    async fn fetch_url(&self, package_path: &str) -> Result<Vec<u8>, CliError> {
        let mut cache_path = cache_location()?;

        let splitter = package_path.split_once(':');
        let Some((_, mut domain_path)) = splitter else {
            return Err(CliError::Cache)
        };

        if domain_path.len() < 2 {
            return Err(CliError::Cache);
        }

        domain_path = &domain_path[2..];

        cache_path.push(PathBuf::from(&domain_path));

        let Some(path) = cache_path.parent() else { return Err(CliError::Cache) };

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

    async fn fetch_catalog(&self, package_name: &str) -> Result<Vec<u8>, CliError> {
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

            let catalog = reqwest::get(&self.catalog).await?;
            let content: serde_json::Value = catalog.json().await?;

            let package_link = &content[&package_name]["source"];
            let Some(package_link) = package_link.as_str() else { return Err(CliError::Catalog) };
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

    fn fetch_local(&self, package_path: &str) -> Result<Vec<u8>, CliError> {
        let path = current_dir()?.join(package_path);

        if !path.exists() {
            return Err(CliError::Local(package_path.to_string()));
        }

        Ok(fs::read(path)?)
    }
}
