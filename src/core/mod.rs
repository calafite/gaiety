pub mod manifest;
pub mod types;

use anyhow::Result;
use std::path::PathBuf;
use types::DiscoveredModule;
use crate::sources::local::LocalSource;
use crate::sources::ModuleSource;
use crate::resolver::graph::sort_modules;
use crate::validator::{validate_commands, validate_any_commands, validate_dependencies};

pub(crate) fn parse_version_lenient(s: &str) -> Result<semver::Version, semver::Error> {
    if let Ok(v) = semver::Version::parse(s) {
        return Ok(v);
    }
    let (base, remainder) = if let Some(idx) = s.find(|c| c == '-' || c == '+') {
        s.split_at(idx)
    } else {
        (s, "")
    };
    let padded_base = match base.split('.').count() {
        1 => format!("{}.0.0", base),
        2 => format!("{}.0", base),
        _ => base.to_string(),
    };
    semver::Version::parse(&format!("{}{}", padded_base, remainder))
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
        let local_source = LocalSource::new(self.dirs.clone());
        let mut modules = local_source.fetch_modules()?;
        sort_modules(&mut modules);
        validate_commands(&mut modules);
        validate_any_commands(&mut modules);
        validate_dependencies(&mut modules);
        Ok(modules)
    }

    pub fn default_write_dir(&self) -> &PathBuf {
        self.dirs.last().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_lenient() {
        assert_eq!(parse_version_lenient("1.2.3").unwrap(), semver::Version::new(1, 2, 3));
        assert_eq!(parse_version_lenient("1.2").unwrap(), semver::Version::new(1, 2, 0));
        assert_eq!(parse_version_lenient("1").unwrap(), semver::Version::new(1, 0, 0));
        assert_eq!(parse_version_lenient("1.2-alpha").unwrap(), semver::Version::parse("1.2.0-alpha").unwrap());
    }

    #[test]
    fn test_loader_new_empty_or_invalid() {
        assert!(Loader::new("").is_err());
        assert!(Loader::new("/nonexistent/path/gaiety/test").is_err());
    }
}
