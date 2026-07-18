use crate::core::loader::Loader;
use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

const TOML_TEMPLATE: &str = include_str!("../templates/module.toml");
const ZSH_TEMPLATE: &str = include_str!("../templates/init.zsh");

pub fn run(directories: String, module_name: String, target: Option<PathBuf>) -> Result<()> {
    if !crate::core::common::name_valid(&module_name) {
        bail!(
            "{}",
            format!(
                "Invalid module name: '{}' (must match [a-zA-Z_][a-zA-Z0-9_]*)",
                module_name
            )
            .red()
        );
    }

    let loader = Loader::new(&directories)?;
    let modules = loader.get_modules()?;

    let matches_module_name = |discovered_module: &crate::core::types::DiscoveredModule| {
        discovered_module.manifest.module.name == module_name
    };
    if modules.iter().any(matches_module_name) {
        bail!(
            "{}",
            format!("Module '{}' is already registered.", module_name).red()
        );
    }

    let default_directory = || loader.default_write().clone();
    let write_directory = target.unwrap_or_else(default_directory);

    let matches_directory = |captured_directory: &PathBuf| captured_directory == &write_directory;
    let write_index = loader.dirs.iter().position(matches_directory);

    let prefix = crate::core::common::next_prefix(&modules, write_index, &write_directory);
    let target_directory_name = format!("{:02}_{}", prefix, module_name);
    let target_directory = write_directory.join(&target_directory_name);

    if target_directory.exists() {
        bail!("Directory already exists: {}", target_directory.display());
    }

    Helper::create_module_files(&target_directory, &module_name)?;

    println!("\n{} {}\n", "::".bold().cyan(), "New Module".bold().cyan());
    println!("  {:<10} {}", "name:".dimmed(), module_name.green());
    println!(
        "  {:<10} {}",
        "dir:".dimmed(),
        target_directory_name.green()
    );
    println!(
        "  {:<10} {}",
        "path:".dimmed(),
        target_directory.display().to_string().dimmed()
    );
    println!("  {:<10} module.toml, init.zsh", "files:".dimmed());
    println!(
        "\n{} Edit the files, then run: {} {}\n",
        "=>".bold().blue(),
        "gai reload".bold(),
        module_name.bold()
    );

    Ok(())
}

struct Helper;

impl Helper {
    fn create_module_files(target_directory: &Path, module_name: &str) -> Result<()> {
        let create_error_context = || {
            format!(
                "Failed to create target directory: {}",
                target_directory.display()
            )
        };
        fs::create_dir_all(target_directory).with_context(create_error_context)?;

        let toml_content = TOML_TEMPLATE.replace("{{MODULE_NAME}}", module_name);
        let toml_path = target_directory.join("module.toml");
        let toml_error_context = || format!("Failed to write: {}", toml_path.display());
        fs::write(&toml_path, toml_content).with_context(toml_error_context)?;

        let zsh_content = ZSH_TEMPLATE.replace("{{MODULE_NAME}}", module_name);
        let zsh_path = target_directory.join("init.zsh");
        let zsh_error_context = || format!("Failed to write: {}", zsh_path.display());
        fs::write(&zsh_path, zsh_content).with_context(zsh_error_context)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_is_valid_name() {
        assert!(crate::core::common::name_valid("my_module"));
        assert!(crate::core::common::name_valid("_private_mod"));
        assert!(crate::core::common::name_valid("mod123"));
        assert!(!crate::core::common::name_valid("123mod"));
        assert!(!crate::core::common::name_valid("my-mod"));
        assert!(!crate::core::common::name_valid(""));
    }
}
