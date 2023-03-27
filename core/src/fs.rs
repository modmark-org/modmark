use std::fmt::Debug;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use wasmer_vfs::mem_fs::FileSystem as MemoryFileSystem;
use wasmer_vfs::{FileSystem, Metadata, OpenOptions, ReadDir};

#[derive(Debug, Clone)]
pub struct MemFS {
    pub inner: Arc<MemoryFileSystem>,
}

impl FileSystem for MemFS {
    fn read_dir(&self, path: &Path) -> wasmer_vfs::Result<ReadDir> {
        self.inner.read_dir(path)
    }

    fn create_dir(&self, path: &Path) -> wasmer_vfs::Result<()> {
        self.inner.create_dir(path)
    }

    fn remove_dir(&self, path: &Path) -> wasmer_vfs::Result<()> {
        self.inner.remove_dir(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> wasmer_vfs::Result<()> {
        self.inner.rename(from, to)
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

impl MemFS {
    pub fn new() -> MemFS {
        MemFS {
            inner: Arc::new(MemoryFileSystem::default()),
        }
    }

    pub fn list_dir(&self, path: &str) -> Vec<(String, bool)> {
        match self.inner.read_dir(Path::new(path)) {
            Ok(entries) => {
                let mut v = vec![];
                for entry_res in entries {
                    let entry = entry_res.unwrap();
                    if let Some(name) = entry.path.file_name() {
                        let filename = name.to_os_string().into_string().unwrap();
                        v.push((format!("{filename}"), entry.file_type().unwrap().dir));
                    }
                }
                v
            }
            _ => vec![],
        }
    }

    pub fn create_file(&self, path: &str, data: &[u8]) -> std::io::Result<()> {
        let mut options = self.new_open_options();
        options.write(true);
        options.create(true);
        options.create_new(true); // TODO: what is the difference between this and create?
        let mut f = options.open(Path::new(path)).unwrap();
        f.write(data)?;
        Ok(())
    }
}
