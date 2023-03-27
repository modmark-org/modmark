use std::fmt::{Debug, Formatter};
use std::fs;
use std::io::{stdin, stdout, Write};
use std::path::Path;
use std::sync::Arc;
use wasmer_vfs::host_fs::{File, FileSystem as HostFileSystem};
use wasmer_vfs::{
    FileOpener, FileSystem, FsError, Metadata, OpenOptions, OpenOptionsConfig, ReadDir, VirtualFile,
};

#[derive(Clone)]
pub struct NativeFs {
    pub inner: Arc<HostFileSystem>,
    pub file_opener: NativeFileOpener,
}

impl Debug for NativeFs {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl NativeFs {
    pub fn new() -> Self {
        NativeFs {
            inner: Arc::new(HostFileSystem::default()),
            file_opener: NativeFileOpener::default(),
        }
    }

    pub fn config(&mut self, deny_read: bool, deny_write: bool, no_prompts: bool) {
        self.file_opener.allow_read = !deny_read;
        self.file_opener.allow_write = !deny_write;
        self.file_opener.no_prompts = no_prompts;
    }

    pub fn set_current_pkg(&mut self, name: &str) {
        self.file_opener.current_pkg = name.to_string();
    }
}

impl FileSystem for NativeFs {
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
        OpenOptions::new(Box::new(self.file_opener.clone()))
    }
}

#[derive(Clone)]
pub struct NativeFileOpener {
    pub current_pkg: String,
    pub allow_read: bool,
    pub allow_write: bool,
    pub no_prompts: bool,
}

impl NativeFileOpener {
    pub fn default() -> Self {
        NativeFileOpener {
            current_pkg: String::new(),
            allow_read: true,
            allow_write: true,
            no_prompts: false,
        }
    }

    fn prompt_user(&self, path: &Path, conf: &OpenOptionsConfig) -> bool {
        let mut permissions = vec![];
        if conf.read() {
            permissions.push("read");
        }
        if conf.write() {
            permissions.push("write");
        }
        if conf.create() {
            permissions.push("create");
        }

        let pkg_str = &self.current_pkg;
        let perms_str = permissions.join(", ");
        let path_str = path.to_str().unwrap();
        print!("Give [{pkg_str}] ({perms_str}) access to {path_str} (y/n): ");
        stdout().flush().expect("Could not flush output");

        loop {
            let mut buffer = String::new();
            stdin().read_line(&mut buffer).expect("");
            let response = buffer.trim().to_lowercase();
            if response == "y" {
                return true;
            } else if response == "n" {
                return false;
            } else {
                print!("Unexpected input. Please enter (y/n): ");
                stdout().flush().expect("Could not flush output");
            }
        }
    }
}

impl FileOpener for NativeFileOpener {
    fn open(
        &mut self,
        path: &Path,
        conf: &OpenOptionsConfig,
    ) -> wasmer_vfs::Result<Box<dyn VirtualFile + Send + Sync + 'static>> {
        if !self.no_prompts && !self.prompt_user(path, conf) {
            return Err(FsError::PermissionDenied);
        }

        let read = conf.read();
        let write = conf.write();
        let append = conf.append();
        let mut oo = fs::OpenOptions::new();
        oo.read(conf.read() && self.allow_read)
            .write(conf.write() && self.allow_write)
            .create_new(conf.create_new() && self.allow_write)
            .create(conf.create() && self.allow_write)
            .append(conf.append())
            .truncate(conf.truncate())
            .open(path)
            .map_err(Into::into)
            .map(|file| {
                Box::new(File::new(file, path.to_owned(), read, write, append))
                    as Box<dyn VirtualFile + Send + Sync + 'static>
            })
    }
}
