use crate::commands::install::is_valid_name;
use crate::core::Loader;
use anyhow::{Result, bail};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

const TOML_TEMPLATE: &str = include_str!("../templates/module.toml");
const ZSH_TEMPLATE: &str = include_str!("../templates/init.zsh");

pub fn run(dirs: String, module_name: String, target: Option<PathBuf>) -> Result<()> {
    if !is_valid_name(&module_name) {
        bail!(
            "{}",
            format!(
                "Invalid module name: '{}' (must match [a-zA-Z_][a-zA-Z0-9_]*)",
                module_name
            )
            .red()
        );
    }

    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    if modules
        .iter()
        .any(|m| m.manifest.module.name == module_name)
    {
        bail!(
            "{}",
            format!("Module '{}' is already registered.", module_name).red()
        );
    }

    let write_dir = target.unwrap_or_else(|| loader.default_write_dir().clone());
    let write_dir_index = loader.dirs.iter().position(|d| d == &write_dir);

    let max_prefix = modules
        .iter()
        .filter(|m| match write_dir_index {
            Some(idx) => m.dir_index == idx,
            None => m.path.parent().is_some_and(|p| p == write_dir),
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

    println!("\n{} {}\n", "::".bold().cyan(), "New Module".bold().cyan());
    println!("  {:<10} {}", "name:".dimmed(), module_name.green());
    println!("  {:<10} {}", "dir:".dimmed(), target_dir_name.green());
    println!(
        "  {:<10} {}",
        "path:".dimmed(),
        target_dir.display().to_string().dimmed()
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

#[cfg(test)]
mod tests {
    use crate::commands::install::is_valid_name;

    #[test]
    fn test_is_valid_name() {
        assert!(is_valid_name("my_module"));
        assert!(is_valid_name("_private_mod"));
        assert!(is_valid_name("mod123"));
        assert!(!is_valid_name("123mod"));
        assert!(!is_valid_name("my-mod"));
        assert!(!is_valid_name(""));
    }
}
