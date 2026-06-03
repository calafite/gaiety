use crate::core::Loader;
use anyhow::{bail, Result};

pub fn run(dirs: String, module_name: String) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let m = modules
        .iter()
        .find(|m| m.manifest.module.name == module_name)
        .ok_or_else(|| anyhow::anyhow!("Module '{}' not found.", module_name))?;

    let init_path = m.path.join("init.zsh");
    if !init_path.exists() {
        bail!("init.zsh not found for module '{}'", module_name);
    }

    println!("{}", init_path.display());
    Ok(())
}
