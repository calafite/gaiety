use crate::loader::Loader;
use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

pub fn run(dirs: String, old_name: String, new_name: String) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let target = modules.iter().find(|m| m.manifest.module.name == old_name);

    let m = match target {
        Some(m) => m,
        None => bail!("Module '{}' not found.", old_name),
    };

    if modules.iter().any(|m| m.manifest.module.name == new_name) {
        bail!("Module '{}' already exists.", new_name);
    }

    let old_dir = &m.path;
    let dir_name = old_dir.file_name().unwrap().to_string_lossy();
    let new_dir_name = match dir_name.splitn(2, '_').collect::<Vec<_>>().as_slice() {
        [prefix, _] => format!("{}_{}", prefix, new_name),
        _ => new_name.clone(),
    };
    let new_dir = old_dir.parent().unwrap().join(&new_dir_name);
    fs::rename(old_dir, &new_dir)?;
    println!("{} renamed dir: {} → {}", "↻".bold().blue(), dir_name, new_dir_name);

    let toml_path = new_dir.join("module.toml");
    let content = fs::read_to_string(&toml_path)?;
    let updated = replace_name_field(&content, &old_name, &new_name);
    fs::write(&toml_path, updated)?;
    println!("{} updated: {}", "↻".bold().blue(), toml_path.display());

    for m in &modules {
        if m.manifest.module.name == old_name {
            continue;
        }
        if m.manifest.module.deps.contains(&old_name) {
            let path = m.path.join("module.toml");
            let content = fs::read_to_string(&path)?;
            let updated = replace_dep(&content, &old_name, &new_name);
            fs::write(&path, updated)?;
            println!("{} updated dep in: {}", "↻".bold().blue(), m.manifest.module.name);
        }
    }

    println!("\n{} renamed '{}' → '{}'\n", "✓".bold().green(), old_name, new_name);
    Ok(())
}

fn replace_name_field(content: &str, old: &str, new: &str) -> String {
    content.replacen(&format!("name        = \"{}\"", old), &format!("name        = \"{}\"", new), 1)
        .replacen(&format!("name = \"{}\"", old), &format!("name = \"{}\"", new), 1)
}

fn replace_dep(content: &str, old: &str, new: &str) -> String {
    content.replace(&format!("\"{}\"", old), &format!("\"{}\"", new))
}use crate::loader::Loader;
use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

pub fn run(dirs: String, old_name: String, new_name: String) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let target = modules.iter().find(|m| m.manifest.module.name == old_name);

    let m = match target {
        Some(m) => m,
        None => bail!("Module '{}' not found.", old_name),
    };

    if modules.iter().any(|m| m.manifest.module.name == new_name) {
        bail!("Module '{}' already exists.", new_name);
    }

    let old_dir = &m.path;
    let dir_name = old_dir.file_name().unwrap().to_string_lossy();
    let new_dir_name = match dir_name.splitn(2, '_').collect::<Vec<_>>().as_slice() {
        [prefix, _] => format!("{}_{}", prefix, new_name),
        _ => new_name.clone(),
    };
    let new_dir = old_dir.parent().unwrap().join(&new_dir_name);
    fs::rename(old_dir, &new_dir)?;
    println!("{} renamed dir: {} → {}", "↻".bold().blue(), dir_name, new_dir_name);

    let toml_path = new_dir.join("module.toml");
    let content = fs::read_to_string(&toml_path)?;
    let updated = replace_name_field(&content, &old_name, &new_name);
    fs::write(&toml_path, updated)?;
    println!("{} updated: {}", "↻".bold().blue(), toml_path.display());

    for m in &modules {
        if m.manifest.module.name == old_name {
            continue;
        }
        if m.manifest.module.deps.contains(&old_name) {
            let path = m.path.join("module.toml");
            let content = fs::read_to_string(&path)?;
            let updated = replace_dep(&content, &old_name, &new_name);
            fs::write(&path, updated)?;
            println!("{} updated dep in: {}", "↻".bold().blue(), m.manifest.module.name);
        }
    }

    println!("\n{} renamed '{}' → '{}'\n", "✓".bold().green(), old_name, new_name);
    Ok(())
}

fn replace_name_field(content: &str, old: &str, new: &str) -> String {
    content.replacen(&format!("name        = \"{}\"", old), &format!("name        = \"{}\"", new), 1)
        .replacen(&format!("name = \"{}\"", old), &format!("name = \"{}\"", new), 1)
}

fn replace_dep(content: &str, old: &str, new: &str) -> String {
    content.replace(&format!("\"{}\"", old), &format!("\"{}\"", new))
}
