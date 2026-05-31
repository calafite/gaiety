use super::types::{DiscoveredModule, ModuleStatus};
use super::Loader;
use crate::manifest::Manifest;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;

impl Loader {
    pub(crate) fn discover_modules(&self) -> Result<Vec<DiscoveredModule>> {
        let mut modules = Vec::new();

        for (dir_index, dir) in self.dirs.iter().enumerate() {
            for entry in fs::read_dir(dir)
                .with_context(|| format!("Failed to read directory: {}", dir.display()))?
            {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    let toml_path = path.join("module.toml");
                    if toml_path.exists() {
                        let content = fs::read_to_string(&toml_path)?;
                        let manifest: Manifest = toml::from_str(&content)
                            .with_context(|| format!("Failed to parse manifest: {}", toml_path.display()))?;

                        let dir_name = path.file_name().unwrap().to_string_lossy();
                        let prefix_order = dir_name
                            .split('_')
                            .next()
                            .and_then(|s| s.parse::<u32>().ok());

                        modules.push(DiscoveredModule {
                            path,
                            manifest,
                            prefix_order,
                            dir_index,
                            status: ModuleStatus::Loaded,
                        });
                    }
                }
            }
        }

        let mut seen: HashMap<String, ()> = HashMap::new();
        modules.reverse();
        modules.retain(|m| seen.insert(m.manifest.module.name.clone(), ()).is_none());
        modules.reverse();

        Ok(modules)
    }

    pub(crate) fn sort_modules(&self, modules: &mut [DiscoveredModule]) {
        modules.sort_by(|a, b| {
            match a.dir_index.cmp(&b.dir_index) {
                std::cmp::Ordering::Equal => match (a.prefix_order, b.prefix_order) {
                    (Some(a_num), Some(b_num)) => a_num.cmp(&b_num),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.manifest.module.name.cmp(&b.manifest.module.name),
                },
                other => other,
            }
        });
    }
}
