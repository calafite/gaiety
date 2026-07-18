use std::{fs, path::PathBuf};

pub struct TempDirGuard {
    path: PathBuf,
    active: bool,
}

impl TempDirGuard {
    pub fn new(path: PathBuf) -> Self {
        Self { path, active: true }
    }

    pub fn defuse(&mut self) {
        self.active = false;
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if self.active && self.path.exists() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

pub struct TempFilesGuard {
    paths: Vec<PathBuf>,
    active: bool,
}

impl TempFilesGuard {
    pub fn new() -> Self {
        Self {
            paths: Vec::new(),
            active: true,
        }
    }

    pub fn add(&mut self, path: PathBuf) {
        self.paths.push(path);
    }

    pub fn defuse(&mut self) {
        self.active = false;
    }
}

impl Drop for TempFilesGuard {
    fn drop(&mut self) {
        if self.active {
            for path in &self.paths {
                let _ = fs::remove_file(path);
            }
        }
    }
}

pub struct TempFileGuard {
    path: PathBuf,
    active: bool,
}

impl TempFileGuard {
    pub fn new(path: PathBuf) -> Self {
        Self { path, active: true }
    }

    pub fn defuse(&mut self) {
        self.active = false;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.active && self.path.exists() {
            let _ = fs::remove_file(&self.path);
        }
    }
}
