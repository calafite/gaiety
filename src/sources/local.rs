use super::ModuleSource;
use crate::core::manifest::Manifest;
use crate::core::types::{DiscoveredModule, ModuleStatus};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub struct LocalSource {
    dirs: Vec<PathBuf>,
}

impl LocalSource {
    pub fn new(dirs: Vec<PathBuf>) -> Self {
        Self { dirs }
    }
}

impl ModuleSource for LocalSource {
    fn fetch_modules(&self) -> Result<Vec<DiscoveredModule>> {
        let mut modules = Vec::new();

        for (directory_index, directory) in self.dirs.iter().enumerate() {
            let entries = fs::read_dir(directory)
                .with_context(|| format!("Failed to read directory: {}", directory.display()))?;

            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() && path.join("module.toml").exists() {
                    modules.push(Helper::read_module(path, directory_index));
                }
            }
        }

        Helper::dedup_modules(&mut modules);
        Ok(modules)
    }
}

struct Helper;

impl Helper {
    fn read_module(path: PathBuf, directory_index: usize) -> DiscoveredModule {
        let toml_path = path.join("module.toml");

        let directory_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "<unknown>".to_string());

        match Self::load_manifest(&toml_path) {
            Ok(manifest) => {
                let prefix_order = Self::prefix_order(&directory_name);
                let status = Self::validate_version(&manifest.module.version);

                DiscoveredModule {
                    path,
                    manifest,
                    prefix_order,
                    dir_index: directory_index,
                    status,
                }
            }
            Err(err) => {
                let placeholder = Manifest::broken(directory_name);
                DiscoveredModule {
                    path,
                    manifest: placeholder,
                    prefix_order: None,
                    dir_index: directory_index,
                    status: ModuleStatus::FailedManifest(err.to_string()),
                }
            }
        }
    }

    fn load_manifest(toml_path: &Path) -> Result<Manifest> {
        let content = fs::read_to_string(toml_path)
            .with_context(|| format!("Failed to parse manifest: {}", toml_path.display()))?;
        toml::from_str::<Manifest>(&content)
            .with_context(|| format!("Failed to parse manifest: {}", toml_path.display()))
    }

    fn prefix_order(directory_name: &str) -> Option<u32> {
        directory_name.split('_').next()?.parse().ok()
    }

    fn validate_version(version: &str) -> ModuleStatus {
        if crate::core::parse_version_lenient(version).is_ok() {
            ModuleStatus::Loaded
        } else {
            ModuleStatus::FailedManifest(format!(
                "invalid version string: '{}' (expected semver, e.g, 1.2.3)",
                version
            ))
        }
    }

    fn dedup_modules(modules: &mut Vec<DiscoveredModule>) {
        let mut seen = HashSet::new();
        modules.reverse();
        modules.retain(|module| seen.insert(module.manifest.module.name.clone()));
        modules.reverse();
    }
}
