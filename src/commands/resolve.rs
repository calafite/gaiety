use crate::commands::install::install_recursive;
use crate::core::loader::Loader;
use crate::core::manifest::Dependency;
use crate::core::types::{DiscoveredModule, ModuleStatus};
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashSet;

pub fn run(directories: String) -> Result<()> {
    let loader_context = || format!("Failed to initialize loader for: {}", directories);
    let loader = Loader::new(&directories).with_context(loader_context)?;

    let modules_context = || "Failed to retrieve modules".to_string();
    let modules = loader.get_modules().with_context(modules_context)?;

    println!(
        "\n{} {}\n",
        "::".bold().cyan(),
        "Resolving Dependencies".bold().cyan()
    );

    let mut visited = HashSet::new();
    let resolved_any = Helper::resolve_missing(&directories, &modules, &mut visited)?;

    if resolved_any {
        println!(
            "\n{} Dependencies resolved successfully.\n",
            "✓".bold().green()
        );
    } else {
        println!("No missing remote dependencies found to resolve.\n");
    }

    Ok(())
}

struct Helper;

impl Helper {
    fn resolve_missing(
        directories: &str,
        modules: &[DiscoveredModule],
        visited: &mut HashSet<String>,
    ) -> Result<bool> {
        let mut resolved_any = false;

        for module in modules {
            match &module.status {
                ModuleStatus::SkippedMissingDep(dependency_name)
                | ModuleStatus::SkippedBadConstraint(dependency_name) => {
                    let matches_dependency_name =
                        |dependency: &&Dependency| &dependency.name == dependency_name;
                    let candidate_dependency = module
                        .manifest
                        .module
                        .deps
                        .iter()
                        .find(matches_dependency_name);

                    if let Some(dependency) = candidate_dependency
                        && let Some(ref source) = dependency.source {
                            println!(
                                "Found remote source for missing dependency '{}' required by '{}': {}",
                                dependency_name.green(),
                                module.manifest.module.name.yellow(),
                                source.dimmed()
                            );
                            install_recursive(
                                directories,
                                source,
                                Some(dependency_name.clone()),
                                None,
                                None,
                                visited,
                            )?;
                            resolved_any = true;
                        }
                }
                _ => {}
            }
        }

        Ok(resolved_any)
    }
}
