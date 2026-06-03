use crate::core::manifest::Manifest;
use crate::core::types::{DiscoveredModule, ModuleStatus};
use super::ModuleSource;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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

        for (dir_index, dir) in self.dirs.iter().enumerate() {
            for entry in fs::read_dir(dir)
                .with_context(|| format!("Failed to read directory: {}", dir.display()))?
            {
                let entry = entry?;
                let path = entry.path();

                if !path.is_dir() {
                    continue;
                }

                let toml_path = path.join("module.toml");
                if !toml_path.exists() {
                    continue;
                }

                let status = match fs::read_to_string(&toml_path)
                    .with_context(|| format!("Failed to read {}", toml_path.display()))
                    .and_then(|content| {
                        toml::from_str::<Manifest>(&content).with_context(|| {
                            format!("Failed to parse manifest: {}", toml_path.display())
                        })
                    }) {
                    Ok(manifest) => {
                        let dir_name = path.file_name().unwrap().to_string_lossy();
                        let prefix_order = dir_name
                            .split('_')
                            .next()
                            .and_then(|s| s.parse::<u32>().ok());

                        let status = if crate::core::parse_version_lenient(&manifest.module.version)
                            .is_ok()
                        {
                            ModuleStatus::Loaded
                        } else {
                            ModuleStatus::FailedManifest(format!(
                                "invalid version string: '{}' (expected semver, e.g. 1.2.3)",
                                manifest.module.version
                            ))
                        };

                        modules.push(DiscoveredModule {
                            path,
                            manifest,
                            prefix_order,
                            dir_index,
                            status,
                        });
                        continue;
                    }
                    Err(e) => ModuleStatus::FailedManifest(e.to_string()),
                };

                let placeholder = Manifest::broken(
                    path.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "<unknown>".to_string()),
                );
                modules.push(DiscoveredModule {
                    path,
                    manifest: placeholder,
                    prefix_order: None,
                    dir_index,
                    status,
                });
            }
        }

        let mut seen: HashMap<String, ()> = HashMap::new();
        modules.reverse();
        modules.retain(|m| seen.insert(m.manifest.module.name.clone(), ()).is_none());
        modules.reverse();

        Ok(modules)
    }
}
