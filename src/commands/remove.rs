use crate::core::loader::Loader;
use crate::core::types::DiscoveredModule;
use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn run(
    directories: String,
    modules_to_remove: Vec<String>,
    directory_filter: Option<PathBuf>,
    recursive: bool,
) -> Result<()> {
    let loader_context = || format!("Failed to initialize loader for: {}", directories);
    let loader = Loader::new(&directories).with_context(loader_context)?;

    let modules_context = || "Failed to retrieve modules".to_string();
    let discovered_modules = loader.get_modules().with_context(modules_context)?;

    let mut unique_names = Vec::new();
    let mut seen_names = HashSet::new();
    for name in modules_to_remove {
        if seen_names.insert(name.clone()) {
            unique_names.push(name);
        }
    }

    let mut target_modules = Vec::new();
    for name in &unique_names {
        let target = Helper::target_module(&discovered_modules, name, &directory_filter)?;
        target_modules.push(target);
    }

    let removals = if recursive {
        Helper::calculate_cascade(&discovered_modules, &unique_names)
    } else {
        unique_names.clone()
    };

    println!(
        "\n{} {}\n",
        "::".bold().cyan(),
        "Remove Modules".bold().cyan()
    );
    println!("  {:<10} {}", "targets:", unique_names.join(", ").green());
    if removals.len() > unique_names.len() {
        let cascading_targets = removals[unique_names.len()..].join(", ");
        println!("  {:<10} {}", "cascading:", cascading_targets.yellow());
    }
    for target in &target_modules {
        println!(
            "  {:<10} {}",
            "path:",
            target.path.display().to_string().dimmed()
        );
    }
    println!();

    print!(
        "{} Remove {} module(s)? [y/N] ",
        "?".bold().yellow(),
        removals.len()
    );

    let flush_context = || "Failed to flush stdout".to_string();
    io::stdout().flush().with_context(flush_context)?;

    let mut input = String::new();
    let read_context = || "Failed to read input from stdin".to_string();
    io::stdin()
        .read_line(&mut input)
        .with_context(read_context)?;

    let confirmed = input.trim().eq_ignore_ascii_case("y");
    if confirmed {
        Helper::perform_removal(&discovered_modules, &removals)?;
        println!("{} removed\n", "✓".bold().green());
    } else {
        println!("{} aborted\n", "!".bold().yellow());
    }

    Ok(())
}

pub fn renumber_modules(directory: &Path) -> Result<()> {
    Helper::renumber_modules(directory)
}

struct Helper;

impl Helper {
    pub fn renumber_modules(directory: &Path) -> Result<()> {
        let read_context = || format!("Failed to read directory: {}", directory.display());

        let unpack_dir_entry = |element: std::io::Result<fs::DirEntry>| element.ok();
        let entry_path = |entry: fs::DirEntry| entry.path();
        let valid_module_directory =
            |path: &PathBuf| path.is_dir() && path.join("module.toml").exists();

        let mut directories: Vec<_> = fs::read_dir(directory)
            .with_context(read_context)?
            .filter_map(unpack_dir_entry)
            .map(entry_path)
            .filter(valid_module_directory)
            .collect();

        directories.sort();

        for (index, path) in directories.iter().enumerate() {
            let directory_name = path.file_name().unwrap().to_string_lossy();
            let suffix = directory_name
                .split_once('_')
                .map(|split| split.1)
                .unwrap_or(&directory_name);
            let new_name = format!("{:02}_{}", index + 1, suffix);

            if directory_name != new_name {
                let rename_context = || {
                    format!(
                        "Failed to rename directory from {} to {}",
                        directory_name, new_name
                    )
                };
                fs::rename(path, directory.join(&new_name)).with_context(rename_context)?;
            }
        }

        Ok(())
    }

    fn target_module<'a>(
        modules: &'a [DiscoveredModule],
        module_name: &str,
        directory_filter: &Option<PathBuf>,
    ) -> Result<&'a DiscoveredModule> {
        let find_predicate = |discovered_module: &&DiscoveredModule| {
            if discovered_module.manifest.module.name != module_name {
                return false;
            }
            match directory_filter {
                Some(filter) => discovered_module.path.parent() == Some(filter.as_path()),
                None => true,
            }
        };
        let candidate = modules.iter().find(find_predicate);

        match candidate {
            Some(discovered_module) => Ok(discovered_module),
            None => match directory_filter {
                Some(filter) => bail!(
                    "Module '{}' not found in '{}'.",
                    module_name,
                    filter.display()
                ),
                None => bail!("Module '{}' not found.", module_name),
            },
        }
    }

    fn calculate_cascade(modules: &[DiscoveredModule], start_names: &[String]) -> Vec<String> {
        let mut removals = start_names.to_vec();

        let mut in_degrees = HashMap::new();
        for module in modules {
            in_degrees
                .entry(module.manifest.module.name.clone())
                .or_insert(0);
            for dependency in &module.manifest.module.deps {
                *in_degrees.entry(dependency.name.clone()).or_insert(0) += 1;
            }
        }

        let mut queue = start_names.to_vec();
        let mut removed_set = HashSet::new();
        for name in start_names {
            removed_set.insert(name.clone());
        }

        while let Some(current_name) = queue.pop() {
            let find_predicate = |discovered_module: &&DiscoveredModule| {
                discovered_module.manifest.module.name == current_name
            };
            if let Some(current_module) = modules.iter().find(find_predicate) {
                for dependency in &current_module.manifest.module.deps {
                    if let Some(degree) = in_degrees.get_mut(&dependency.name) {
                        if *degree > 0 {
                            *degree -= 1;
                        }
                        let orphaned = *degree == 0 && !removed_set.contains(&dependency.name);
                        if orphaned {
                            removed_set.insert(dependency.name.clone());
                            queue.push(dependency.name.clone());
                            removals.push(dependency.name.clone());
                        }
                    }
                }
            }
        }

        removals
    }

    fn perform_removal(modules: &[DiscoveredModule], to_remove: &[String]) -> Result<()> {
        let mut affected_directories = HashSet::new();
        for name in to_remove {
            let find_predicate = |discovered_module: &&DiscoveredModule| {
                discovered_module.manifest.module.name == *name
            };
            if let Some(target_module) = modules.iter().find(find_predicate) {
                if let Some(parent_directory) = target_module.path.parent() {
                    affected_directories.insert(parent_directory.to_path_buf());
                }
                let remove_context = || {
                    format!(
                        "Failed to remove module directory: {}",
                        target_module.path.display()
                    )
                };
                fs::remove_dir_all(&target_module.path).with_context(remove_context)?;
            }
        }
        for directory in affected_directories {
            Self::renumber_modules(&directory)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_temp_directory(name: &str) -> PathBuf {
        let temp_dir_path = std::env::temp_dir();
        let temporary_directory_name = crate::core::common::temporary_name(name);
        let target_path = temp_dir_path.join(temporary_directory_name);
        fs::create_dir_all(&target_path).unwrap();
        target_path
    }

    #[test]
    fn test_renumber_modules() {
        let temp = create_temp_directory("renumber");
        let module1 = temp.join("05_foo");
        let module2 = temp.join("12_bar");
        fs::create_dir_all(&module1).unwrap();
        fs::create_dir_all(&module2).unwrap();
        fs::write(module1.join("module.toml"), "").unwrap();
        fs::write(module2.join("module.toml"), "").unwrap();

        renumber_modules(&temp).unwrap();

        assert!(temp.join("01_foo").exists());
        assert!(temp.join("02_bar").exists());

        let _ = fs::remove_dir_all(&temp);
    }
}
