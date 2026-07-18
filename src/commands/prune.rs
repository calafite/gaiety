use crate::core::loader::Loader;
use crate::core::types::DiscoveredModule;
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};

pub fn run(directories: String) -> Result<()> {
    let loader_context = || format!("Failed to initialize loader for: {}", directories);
    let loader = Loader::new(&directories).with_context(loader_context)?;

    let modules_context = || "Failed to retrieve modules".to_string();
    let modules = loader.get_modules().with_context(modules_context)?;

    let orphaned_modules = Helper::orphaned_modules(&modules);

    if orphaned_modules.is_empty() {
        println!("\nNo orphaned dependencies to prune.\n");
        return Ok(());
    }

    println!(
        "\n{} {}\n",
        "::".bold().cyan(),
        "Prune Orphaned Dependencies".bold().cyan()
    );
    for module in &orphaned_modules {
        println!(
            "  {:<14} {}",
            module.manifest.module.name.red(),
            module.path.display().to_string().dimmed()
        );
    }
    println!();

    print!(
        "{} Remove {} orphaned module(s)? [y/N] ",
        "?".bold().yellow(),
        orphaned_modules.len()
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
        Helper::perform_prune(&orphaned_modules)?;
        println!(
            "{} pruned {} module(s)\n",
            "✓".bold().green(),
            orphaned_modules.len()
        );
    } else {
        println!("{} aborted\n", "!".bold().yellow());
    }

    Ok(())
}

struct Helper;

impl Helper {
    fn orphaned_modules(modules: &[DiscoveredModule]) -> Vec<DiscoveredModule> {
        let mut orphaned_modules = Vec::new();
        let mut pruned_set = HashSet::new();

        loop {
            let mut in_degrees = HashMap::new();
            for module in modules {
                if pruned_set.contains(&module.manifest.module.name) {
                    continue;
                }
                in_degrees
                    .entry(module.manifest.module.name.clone())
                    .or_insert(0);
                for dependency in &module.manifest.module.deps {
                    if !pruned_set.contains(&dependency.name) {
                        *in_degrees.entry(dependency.name.clone()).or_insert(0) += 1;
                    }
                }
            }

            let mut found_any = false;
            for module in modules {
                let name = &module.manifest.module.name;
                if pruned_set.contains(name) {
                    continue;
                }

                let implicit = module.manifest.module.implicit == Some(true);
                let degree_zero = in_degrees.get(name).copied() == Some(0);

                if implicit && degree_zero {
                    orphaned_modules.push(module.clone());
                    pruned_set.insert(name.clone());
                    found_any = true;
                }
            }

            if !found_any {
                break;
            }
        }
        orphaned_modules
    }

    fn perform_prune(orphaned_modules: &[DiscoveredModule]) -> Result<()> {
        let mut affected_directories = HashSet::new();
        for module in orphaned_modules {
            if let Some(parent_directory) = module.path.parent() {
                affected_directories.insert(parent_directory.to_path_buf());
            }
            let remove_error_context = || {
                format!(
                    "Failed to remove orphaned module directory: {}",
                    module.path.display()
                )
            };
            fs::remove_dir_all(&module.path).with_context(remove_error_context)?;
        }
        for directory in affected_directories {
            let renumber_error_context = || {
                format!(
                    "Failed to renumber modules in directory: {}",
                    directory.display()
                )
            };
            super::remove::renumber_modules(&directory).with_context(renumber_error_context)?;
        }
        Ok(())
    }
}
