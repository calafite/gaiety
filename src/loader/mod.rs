pub mod types;
mod discover;
mod emit;
mod validate;

use anyhow::Result;
use std::path::PathBuf;
use types::DiscoveredModule;

pub(crate) fn parse_version_lenient(s: &str) -> Result<semver::Version, semver::Error> {
    if let Ok(v) = semver::Version::parse(s) {
        return Ok(v);
    }
    match s.split('.').count() {
        1 => semver::Version::parse(&format!("{}.0.0", s)),
        2 => semver::Version::parse(&format!("{}.0", s)),
        _ => semver::Version::parse(s),
    }
}

pub struct Loader {
    pub dirs: Vec<PathBuf>,
}

impl Loader {
    pub fn new(dirs: &str) -> Result<Self> {
        let dirs = dirs
            .split(':')
            .filter(|s| !s.is_empty())
            .map(|s| {
                let expanded = shellexpand::tilde(s);
                let path = PathBuf::from(expanded.as_ref());
                if !path.exists() {
                    anyhow::bail!("module directory does not exist: {}", path.display());
                }
                Ok(path)
            })
            .collect::<Result<Vec<_>>>()?;

        if dirs.is_empty() {
            anyhow::bail!("no module directories specified");
        }

        Ok(Self { dirs })
    }

    pub fn get_modules(&self) -> Result<Vec<DiscoveredModule>> {
        let mut modules = self.discover_modules()?;
        self.sort_modules(&mut modules);
        self.validate_commands(&mut modules);
        self.validate_any_commands(&mut modules);
        self.validate_dependencies(&mut modules);
        Ok(modules)
    }

    pub fn default_write_dir(&self) -> &PathBuf {
        self.dirs.last().unwrap()
    }
}
