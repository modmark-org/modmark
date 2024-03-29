use crate::AccessPolicy;
use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::sync::{Arc, Mutex};
#[cfg(feature = "native")]
use wasmer_vfs::host_fs::{FileOpener as HostFileOpener, FileSystem as HostFileSystem};
#[cfg(feature = "web")]
use wasmer_vfs::mem_fs::FileSystem as MemoryFileSystem;
#[cfg(feature = "native")]
use wasmer_vfs::{FileOpener, OpenOptionsConfig, VirtualFile};
use wasmer_vfs::{FileSystem, FsError, Metadata, OpenOptions, ReadDir};

pub struct CoreFs<T> {
    #[cfg(feature = "native")]
    inner: Arc<HostFileSystem>,
    #[cfg(feature = "web")]
    inner: Arc<MemoryFileSystem>,
    file_opener: CoreFileOpener<T>,
}

impl<T> Debug for CoreFs<T> {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<T> FileSystem for CoreFs<T>
where
    T: AccessPolicy,
{
    fn read_dir(&self, path: &Path) -> wasmer_vfs::Result<ReadDir> {
        self.inner.read_dir(path)
    }

    fn create_dir(&self, path: &Path) -> wasmer_vfs::Result<()> {
        // This check for duplicates is required since MemoryFileSystem
        // does not seem to properly handle duplicates itself
        if let Some(parent) = path.parent() {
            for (name, _) in self.list_dir(parent)? {
                if path.ends_with(name.as_str()) {
                    return Err(FsError::AddressInUse);
                }
            }
        } else {
            return Err(FsError::InvalidInput);
        }
        self.inner.create_dir(path)
    }

    fn remove_dir(&self, path: &Path) -> wasmer_vfs::Result<()> {
        self.inner.remove_dir(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> wasmer_vfs::Result<()> {
        // This check for duplicates is required since MemoryFileSystem
        // does not seem to properly handle duplicates itself
        if let Some(parent) = from.parent() {
            for (name, _) in self.list_dir(parent)? {
                if to.ends_with(name.as_str()) {
                    return Err(FsError::AddressInUse);
                }
            }
        } else {
            return Err(FsError::InvalidInput);
        }
        self.inner.rename(from, to)
    }

    fn metadata(&self, path: &Path) -> wasmer_vfs::Result<Metadata> {
        self.inner.metadata(path)
    }

    fn remove_file(&self, path: &Path) -> wasmer_vfs::Result<()> {
        self.inner.remove_file(path)
    }

    fn new_open_options(&self) -> OpenOptions {
        #[cfg(feature = "native")]
        {
            OpenOptions::new(Box::new(self.file_opener.clone()))
        }
        #[cfg(feature = "web")]
        {
            self.inner.new_open_options()
        }
        // TODO: Make use of AccessPolicy when MemoryFileSystem implements FileOpener (v.3.2.0)
    }
}

impl<T> CoreFs<T>
where
    T: AccessPolicy,
{
    pub(crate) fn new(policy: Arc<Mutex<T>>) -> Self {
        #[cfg(feature = "native")]
        let inner = Arc::new(HostFileSystem::default());
        #[cfg(feature = "web")]
        let inner = Arc::new(MemoryFileSystem::default());

        Self {
            inner,
            file_opener: CoreFileOpener::new(policy),
        }
    }
}

impl<T> CoreFs<T> {
    pub(crate) fn clone_for_module(&self, name: String) -> Self {
        let mut file_opener = self.file_opener.clone();
        file_opener.current_module = name;
        Self {
            inner: self.inner.clone(),
            file_opener,
        }
    }
}

impl<T> CoreFs<T>
where
    T: AccessPolicy,
{
    pub fn list_dir(&self, path: &Path) -> wasmer_vfs::Result<Vec<(String, bool)>> {
        let mut v = vec![];
        match self.inner.read_dir(path) {
            Ok(entries) => {
                // fine to unwrap the results in DirEntry here, source code always gives Ok()
                for entry in entries.map(|res| res.unwrap()) {
                    let name = entry.file_name().into_string().unwrap();
                    let is_folder = entry.file_type().unwrap().dir;
                    v.push((name, is_folder));
                }
            }
            _ => {
                return Err(FsError::BaseNotDirectory);
            }
        }
        Ok(v)
    }

    // checking for duplicates here shouldn't be necessary since we'll overwrite if it exists
    pub fn create_file(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        let mut options = self.new_open_options();
        options.write(true).create_new(true);
        options.open(path).and_then(|mut f| {
            f.write_all(data)?;
            Ok(())
        })
    }

    pub fn read_file(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        let mut options = self.new_open_options();
        options.read(true);
        options.open(path).and_then(|mut f| {
            let mut buf = vec![];
            f.read_to_end(&mut buf)?;
            Ok(buf)
        })
    }
}

pub struct CoreFileOpener<T> {
    policy: Arc<Mutex<T>>,
    current_module: String,
}

impl<T> Clone for CoreFileOpener<T> {
    fn clone(&self) -> Self {
        Self {
            policy: self.policy.clone(),
            current_module: self.current_module.clone(),
        }
    }
}

impl<T> CoreFileOpener<T>
where
    T: AccessPolicy,
{
    fn new(policy: Arc<Mutex<T>>) -> Self {
        Self {
            policy,
            current_module: String::new(),
        }
    }

    #[cfg(feature = "native")]
    fn handle_options_config(
        &mut self,
        path: &Path,
        conf: &OpenOptionsConfig,
    ) -> OpenOptionsConfig {
        let mut policy = self.policy.lock().unwrap();

        let read = conf.read()
            && policy.allowed_to_read()
            && policy.allowed_access(path, &self.current_module);
        let write = conf.write()
            && policy.allowed_to_write()
            && policy.allowed_access(path, &self.current_module);
        let create = conf.create()
            && policy.allowed_to_create()
            && policy.allowed_access(path, &self.current_module);

        OpenOptionsConfig {
            read,
            write,
            create_new: create,
            create,
            append: conf.append(),
            truncate: conf.truncate(),
        }
    }
}

#[cfg(feature = "native")]
impl<T> FileOpener for CoreFileOpener<T>
where
    T: AccessPolicy,
{
    fn open(
        &mut self,
        path: &Path,
        conf: &OpenOptionsConfig,
    ) -> wasmer_vfs::Result<Box<dyn VirtualFile + Send + Sync + 'static>> {
        let mut opener = HostFileOpener;
        opener.open(path, &self.handle_options_config(path, conf))
    }
}
