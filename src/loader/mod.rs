pub mod types;
mod discover;
mod emit;
mod validate;

use anyhow::Result;
use std::path::PathBuf;
use types::DiscoveredModule;

pub struct Loader {
    pub dir: PathBuf,
}

impl Loader {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn get_modules(&self) -> Result<Vec<DiscoveredModule>> {
        let mut modules = self.discover_modules()?;
        self.sort_modules(&mut modules);
        self.validate_commands(&mut modules);
        self.validate_dependencies(&mut modules);
        Ok(modules)
    }
}
