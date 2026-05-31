use crate::loader::Loader;
use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

pub fn run(dirs: String, module_name: String, target: Option<PathBuf>) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let target_module = modules.iter().find(|m| m.manifest.module.name == module_name);

    let m = match target_module {
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
        let module_parent = m.path.parent().unwrap().to_path_buf();
        fs::remove_dir_all(&m.path)?;
        println!("{} deleted: {}", "✓".bold().green(), m.path.display());
        let write_dir = target.unwrap_or(module_parent);
        renumber_modules(&write_dir)?;
    } else {
        println!("{} aborted", "!".bold().yellow());
    }

    println!();
    Ok(())
}

fn renumber_modules(dir: &PathBuf) -> Result<()> {
    let mut dirs: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.join("module.toml").exists())
        .collect();

    dirs.sort();

    for (i, path) in dirs.iter().enumerate() {
        let dir_name = path.file_name().unwrap().to_string_lossy();
        let suffix = dir_name.splitn(2, '_').nth(1).unwrap_or(&dir_name);
        let new_name = format!("{:02}_{}", i + 1, suffix);
        if dir_name != new_name {
            fs::rename(path, dir.join(&new_name))?;
            println!("{} renamed: {} → {}", "↻".bold().blue(), dir_name, new_name);
        }
    }

    Ok(())
}
