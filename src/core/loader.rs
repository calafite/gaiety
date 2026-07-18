use crate::resolver::graph::Sorter;
use crate::validator::commands::CommandValidator;
use crate::validator::semver::DependencyValidator;
use crate::{
    core::types::DiscoveredModule,
    sources::{ModuleSource, local::LocalSource},
};
use anyhow::Result;
use std::path::PathBuf;

pub struct Loader {
    pub dirs: Vec<PathBuf>,
}

impl Loader {
    pub fn new(dirs: &str) -> Result<Self> {
        let dirs = dirs
            .split(':')
            .filter(|string| !string.is_empty())
            .map(Self::parse_validate)
            .collect::<Result<Vec<_>>>()?;

        if dirs.is_empty() {
            anyhow::bail!("no module directories specified");
        }

        Ok(Self { dirs })
    }

    pub fn get_modules(&self) -> Result<Vec<DiscoveredModule>> {
        let local_source = LocalSource::new(self.dirs.clone());
        let mut modules = local_source.fetch_modules()?;
        Sorter::sort_modules(&mut modules);
        CommandValidator::cmds(&mut modules);
        CommandValidator::any_cmds(&mut modules);
        DependencyValidator::validate(&mut modules);
        Ok(modules)
    }

    pub fn default_write(&self) -> &PathBuf {
        self.dirs
            .last()
            .expect("Loader invariant violated: directories cannot be empty after initialization.")
    }

    fn parse_validate(path: &str) -> Result<PathBuf> {
        let expanded = shellexpand::tilde(path);
        let path = PathBuf::from(expanded.as_ref());

        if !path.exists() {
            anyhow::bail!("module directory does not exist: {}", path.display());
        }

        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_new_empty_or_invalid() {
        assert!(Loader::is_err(&Loader::new("")));
        assert!(Loader::is_err(&Loader::new(
            "/nonexistent/path/gaiety/test"
        )));
    }

    impl Loader {
        fn is_err<T>(res: &Result<T>) -> bool {
            res.is_err()
        }
    }
}
