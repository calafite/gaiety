use crate::core::Loader;
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use toml_edit::DocumentMut;

pub fn run(dirs: String, module_name: String) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let m = modules
        .iter()
        .find(|m| m.manifest.module.name == module_name)
        .ok_or_else(|| anyhow::anyhow!("Module '{}' not found.", module_name))?;

    if m.manifest.module.enabled == Some(false) {
        println!("Module '{}' is already disabled.", module_name);
        return Ok(());
    }

    let toml_path = m.path.join("module.toml");
    let content = fs::read_to_string(&toml_path)
        .with_context(|| format!("Failed to read {}", toml_path.display()))?;

    let mut doc = content
        .parse::<DocumentMut>()
        .with_context(|| format!("Failed to parse {}", toml_path.display()))?;

    // Defensively fetch or insert the [module] table to prevent panics
    let module_entry = doc
        .entry("module")
        .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));

    if let Some(table) = module_entry.as_table_mut() {
        table.insert("enabled", toml_edit::value(false));
    } else {
        bail!("Invalid manifest: [module] is not a table");
    }

    fs::write(&toml_path, doc.to_string())
        .with_context(|| format!("Failed to write {}", toml_path.display()))?;

    println!("{} disabled '{}'\n", "✓".bold().green(), module_name);
    println!(
        "{} Run {} to apply changes to your current session\n",
        "=>".bold().blue(),
        "gai reload".bold()
    );

    Ok(())
}
