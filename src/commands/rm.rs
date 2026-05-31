use crate::loader::Loader;
use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

pub fn run(dir: PathBuf, module_name: String) -> Result<()> {
    if module_name == "core" {
        bail!("Cannot remove 'core' — it is usually required by the runtime.");
    }

    let loader = Loader::new(dir);
    let modules = loader.get_modules()?;

    let target = modules.iter().find(|m| m.manifest.module.name == module_name);

    let m = match target {
        Some(m) => m,
        None => bail!("Module '{}' not found.", module_name),
    };

    println!("\n{} {}\n", "::".bold().cyan(), format!("Remove Module: {}", module_name).bold().cyan());
    println!("  {} {}\n", "path:".dimmed(), m.path.display());

    print!("{} Remove module '{}'? [y/N] ", "?".bold().yellow(), module_name);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().eq_ignore_ascii_case("y") {
        fs::remove_dir_all(&m.path)?;
        println!("{} deleted: {}", "✓".bold().green(), m.path.display());
    } else {
        println!("{} aborted", "!".bold().yellow());
    }

    println!();
    Ok(())
}
