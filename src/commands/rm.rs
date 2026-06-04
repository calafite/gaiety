use crate::core::Loader;
use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

pub fn run(
    dirs: String,
    module_name: String,
    dir_filter: Option<PathBuf>,
    recursive: bool,
) -> Result<()> {
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

    let mut to_remove = vec![m.manifest.module.name.clone()];

    if recursive {
        // Build dependency graph to find cascading orphans
        let mut in_degrees: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for mod_item in &modules {
            in_degrees.entry(mod_item.manifest.module.name.clone()).or_insert(0);
            for dep in &mod_item.manifest.module.deps {
                *in_degrees.entry(dep.name.clone()).or_insert(0) += 1;
            }
        }

        // We simulate removal of the target and its dependencies recursively
        let mut queue = vec![m.manifest.module.name.clone()];
        let mut removed_set: std::collections::HashSet<String> = std::collections::HashSet::new();
        removed_set.insert(m.manifest.module.name.clone());

        while let Some(curr) = queue.pop() {
            if let Some(curr_mod) = modules.iter().find(|x| x.manifest.module.name == curr) {
                for dep in &curr_mod.manifest.module.deps {
                    if let Some(deg) = in_degrees.get_mut(&dep.name) {
                        if *deg > 0 {
                            *deg -= 1;
                        }
                        if *deg == 0 && !removed_set.contains(&dep.name) {
                            removed_set.insert(dep.name.clone());
                            queue.push(dep.name.clone());
                            to_remove.push(dep.name.clone());
                        }
                    }
                }
            }
        }
    }

    println!("\n{} {}\n", "::".bold().cyan(), "Remove Module".bold().cyan());
    println!("  {:<10} {}", "target:".dimmed(), module_name.green());
    if to_remove.len() > 1 {
        println!("  {:<10} {}", "cascading:".dimmed(), to_remove[1..].join(", ").yellow());
    }
    println!("  {:<10} {}\n", "path:".dimmed(), m.path.display().to_string().dimmed());

    print!(
        "{} Remove {} module(s)? [y/N] ",
        "?".bold().yellow(),
        to_remove.len()
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().eq_ignore_ascii_case("y") {
        let mut affected_dirs = std::collections::HashSet::new();
        for name in to_remove {
            if let Some(target_mod) = modules.iter().find(|x| x.manifest.module.name == name) {
                if let Some(parent) = target_mod.path.parent() {
                    affected_dirs.insert(parent.to_path_buf());
                }
                fs::remove_dir_all(&target_mod.path)?;
            }
        }
        for dir in affected_dirs {
            renumber_modules(&dir)?;
        }
        println!("{} removed\n", "✓".bold().green());
    } else {
        println!("{} aborted\n", "!".bold().yellow());
    }

    Ok(())
}

pub fn renumber_modules(dir: &PathBuf) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_temp_dir(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("gai_test_rm_{}_{}", name, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros()));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn test_renumber_modules() {
        let temp = create_temp_dir("renumber");
        let m1 = temp.join("05_foo");
        let m2 = temp.join("12_bar");
        fs::create_dir_all(&m1).unwrap();
        fs::create_dir_all(&m2).unwrap();
        fs::write(m1.join("module.toml"), "").unwrap();
        fs::write(m2.join("module.toml"), "").unwrap();

        renumber_modules(&temp).unwrap();

        assert!(temp.join("01_foo").exists());
        assert!(temp.join("02_bar").exists());

        let _ = fs::remove_dir_all(&temp);
    }
}
