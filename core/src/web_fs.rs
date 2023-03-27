use std::fmt::Debug;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use wasmer_vfs::mem_fs::FileSystem as MemoryFileSystem;
use wasmer_vfs::{FileSystem, FsError, Metadata, OpenOptions, ReadDir};

#[derive(Debug, Clone)]
pub struct WebFs {
    pub inner: Arc<MemoryFileSystem>,
}

impl FileSystem for WebFs {
    fn read_dir(&self, path: &Path) -> wasmer_vfs::Result<ReadDir> {
        self.inner.read_dir(path)
    }

    fn create_dir(&self, path: &Path) -> wasmer_vfs::Result<()> {
        if let Some(parent) = path.parent() {
            for (name, _) in self.list_dir(parent)? {
                if path.ends_with(name.as_str()) {
                    return Err(FsError::AddressInUse);
                }
            }
            self.inner.create_dir(path)
        } else {
            Err(FsError::InvalidInput)
        }
    }

    fn remove_dir(&self, path: &Path) -> wasmer_vfs::Result<()> {
        self.inner.remove_dir(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> wasmer_vfs::Result<()> {
        if let Some(parent) = from.parent() {
            for (name, _) in self.list_dir(parent)? {
                if to.ends_with(name.as_str()) {
                    return Err(FsError::AddressInUse);
                }
            }
            self.inner.rename(from, to)
        } else {
            Err(FsError::InvalidInput)
        }
    }

    fn metadata(&self, path: &Path) -> wasmer_vfs::Result<Metadata> {
        self.inner.metadata(path)
    }

    fn remove_file(&self, path: &Path) -> wasmer_vfs::Result<()> {
        self.inner.remove_file(path)
    }

    fn new_open_options(&self) -> OpenOptions {
        self.inner.new_open_options()
    }
}

impl WebFs {
    pub fn new() -> WebFs {
        WebFs {
            inner: Arc::new(MemoryFileSystem::default()),
        }
    }

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
    pub fn create_file(&self, path: &Path, data: &[u8]) -> std::io::Result<()> {
        let mut options = self.new_open_options();
        options.write(true).create(true);
        let mut f = options.open(path).unwrap();
        f.write(data)?;
        Ok(())
    }

    pub fn read_file(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        let mut options = self.new_open_options();
        options.read(true);
        let mut f = options.open(path).unwrap();
        let mut buf = vec![];
        f.read_to_end(&mut buf)?;
        Ok(buf)
    }
}
