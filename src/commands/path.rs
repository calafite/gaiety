use crate::core::loader::Loader;
use crate::core::types::DiscoveredModule;
use anyhow::{Context, Result, anyhow, bail};
use std::path::PathBuf;

pub fn run(directories: String, module_name: String) -> Result<()> {
    let loader_context = || format!("Failed to initialize loader for: {}", directories);
    let loader = Loader::new(&directories).with_context(loader_context)?;

    let modules_context = || "Failed to retrieve modules".to_string();
    let modules = loader.get_modules().with_context(modules_context)?;

    let init_path = Helper::module_init_path(&modules, &module_name)?;

    println!("{}", init_path.display());
    Ok(())
}

struct Helper;

impl Helper {
    fn module_init_path(modules: &[DiscoveredModule], module_name: &str) -> Result<PathBuf> {
        let find_predicate = |discovered_module: &&DiscoveredModule| {
            discovered_module.manifest.module.name == module_name
        };
        let target_module = modules
            .iter()
            .find(find_predicate)
            .ok_or_else(|| anyhow!("Module '{}' not found.", module_name))?;

        let init_path = target_module.path.join("init.zsh");
        let file_exists = init_path.is_file();
        if !file_exists {
            bail!("init.zsh not found for module '{}'", module_name);
        }

        Ok(init_path)
    }
}
