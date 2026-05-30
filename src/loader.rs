use crate::manifest::Manifest;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct DiscoveredModule {
    pub path: PathBuf,
    pub manifest: Manifest,
    pub prefix_order: Option<u32>,
}

pub struct Loader {
    dir: PathBuf,
}

impl Loader {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// The main orchestration method
    pub fn run(&self) -> Result<()> {
        println!("# ZRT Loader initializing from: {}", self.dir.display());

        let mut modules = self.discover_modules()?;
        self.sort_modules(&mut modules);

        for mod_info in &modules {
            println!(
                "# Discovered module: {} (Order: {:?})",
                mod_info.manifest.module.name, mod_info.prefix_order
            );
        }

        Ok(())
    }

    fn discover_modules(&self) -> Result<Vec<DiscoveredModule>> {
        let mut modules = Vec::new();

        for entry in fs::read_dir(&self.dir).context("Failed to read modules directory")? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let toml_path = path.join("module.toml");
                if toml_path.exists() {
                    let content = fs::read_to_string(&toml_path)
                        .with_context(|| format!("Failed to read {:?}", toml_path))?;

                    let manifest: Manifest = toml::from_str(&content)
                        .with_context(|| format!("Failed to parse {:?}", toml_path))?;

                    // Extract the NN_ prefix if it exists (e.g., "03_list" -> Some(3))
                    let dir_name = path.file_name().unwrap().to_string_lossy();
                    let prefix_order = dir_name
                        .split('_')
                        .next()
                        .and_then(|s| s.parse::<u32>().ok());

                    modules.push(DiscoveredModule {
                        path,
                        manifest,
                        prefix_order,
                    });
                }
            }
        }

        Ok(modules)
    }

    fn sort_modules(&self, modules: &mut [DiscoveredModule]) {
        modules.sort_by(|a, b| match (a.prefix_order, b.prefix_order) {
            (Some(a_num), Some(b_num)) => a_num.cmp(&b_num),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.manifest.module.name.cmp(&b.manifest.module.name),
        });
    }
}
