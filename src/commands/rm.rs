use crate::loader::Loader;
use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

pub fn run(dirs: String, module_name: String, dir_filter: Option<PathBuf>) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let candidate = modules.iter().find(|m| {
        if m.manifest.module.name != module_name {
            return false;
        }
        match &dir_filter {
            Some(filter) => m.path.parent().map_or(false, |p| p == filter),
            None => true,
        }
    });

    let m = match candidate {
        Some(m) => m,
        None => match &dir_filter {
            Some(filter) => bail!(
                "Module '{}' not found in '{}'.",
                module_name,
                filter.display()
            ),
            None => bail!("Module '{}' not found.", module_name),
        },
    };

    println!("\n{} {}\n", "::".bold().cyan(), "Remove Module".bold().cyan());
    println!("  {:<10} {}", "name:".dimmed(), module_name.green());
    println!("  {:<10} {}\n", "path:".dimmed(), m.path.display().to_string().dimmed());

    print!(
        "{} Remove module '{}'? [y/N] ",
        "?".bold().yellow(),
        module_name
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().eq_ignore_ascii_case("y") {
        let module_dir = m.path.parent().unwrap().to_path_buf();
        fs::remove_dir_all(&m.path)?;
        renumber_modules(&module_dir)?;
        println!("{} removed\n", "✓".bold().green());
    } else {
        println!("{} aborted\n", "!".bold().yellow());
    }

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
        }
    }

    Ok(())
}
