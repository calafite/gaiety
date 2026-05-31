use crate::loader::Loader;
use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

const TOML_TEMPLATE: &str = include_str!("../templates/module.toml");
const ZSH_TEMPLATE: &str = include_str!("../templates/init.zsh");

pub fn run(dirs: String, module_name: String, target: Option<PathBuf>) -> Result<()> {
    if !is_valid_name(&module_name) {
        bail!("{}", format!(
            "Invalid module name: '{}' (must match [a-zA-Z_][a-zA-Z0-9_]*)",
            module_name
        ).red());
    }

    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    if modules.iter().any(|m| m.manifest.module.name == module_name) {
        bail!("{}", format!("Module '{}' is already registered.", module_name).red());
    }

    let write_dir = target.unwrap_or_else(|| loader.default_write_dir().clone());
    let write_dir_index = loader.dirs.iter().position(|d| d == &write_dir);

    let max_prefix = modules
        .iter()
        .filter(|m| match write_dir_index {
            Some(idx) => m.dir_index == idx,
            None => m.path.parent().map_or(false, |p| p == write_dir),
        })
        .filter_map(|m| m.prefix_order)
        .max()
        .unwrap_or(0);

    let next_prefix = format!("{:02}", max_prefix + 1);
    let target_dir_name = format!("{}_{}", next_prefix, module_name);
    let target_dir = write_dir.join(&target_dir_name);

    if target_dir.exists() {
        bail!("Directory already exists: {}", target_dir.display());
    }

    fs::create_dir_all(&target_dir)?;

    let toml_content = TOML_TEMPLATE.replace("{{MODULE_NAME}}", &module_name);
    fs::write(target_dir.join("module.toml"), toml_content)?;

    let zsh_content = ZSH_TEMPLATE.replace("{{MODULE_NAME}}", &module_name);
    fs::write(target_dir.join("init.zsh"), zsh_content)?;

    println!("\n{} {}\n", "::".bold().cyan(), "Module Created".bold().cyan());
    println!("  {:<10} {}", "name:".dimmed(), module_name.green());
    println!("  {:<10} {}", "dir:".dimmed(), target_dir_name.green());
    println!("  {:<10} {}", "path:".dimmed(), target_dir.display().to_string().dimmed());
    println!("  {:<10} {}", "files:".dimmed(), "module.toml, init.zsh");
    println!(
        "\n{} Edit the files, then run: {} {}\n",
        "=>".bold().blue(),
        "gai reload".bold(),
        module_name.bold()
    );

    Ok(())
}

fn is_valid_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
