use std::fmt::Debug;
use std::io::{Read, Write};
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

    pub fn list_dir(&self, path: &Path) -> Vec<(String, bool)> {
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
            _ => {}
        }
        v
    }

    pub fn create_file(&self, path: &Path, data: &[u8]) -> std::io::Result<()> {
        let mut options = self.new_open_options();
        options.write(true);
        options.create(true);
        options.create_new(true); // TODO: what is the difference between this and create?
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
