use std::{env::current_dir, fs, path::PathBuf};

use directories::ProjectDirs;

pub trait Resolve {
    fn resolve(&self, path: &str) -> Option<Vec<u8>>;
}

pub struct PackageManager;
impl Resolve for PackageManager {
    fn resolve(&self, path: &str) -> Option<Vec<u8>> {
        let mut local_path = match current_dir() {
            Ok(p) => p,
            Err(_) => return None,
        };

        local_path.push(PathBuf::from(path));

        let mut cache_path = match ProjectDirs::from("org", "modmark", "packages") {
            Some(path) => path.cache_dir().to_path_buf(),
            None => return None,
        };

        cache_path.push(PathBuf::from(path));

        if local_path.exists() {
            fs::read(local_path).ok();
        }

        if cache_path.exists() {
            fs::read(cache_path).ok();
        }

        return None;
    }
}
