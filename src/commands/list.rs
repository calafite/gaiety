use crate::core::loader::Loader;
use crate::core::manifest::Dependency;
use crate::core::types::{DiscoveredModule, ModuleStatus};
use anyhow::{Context, Result};
use colored::Colorize;

pub fn run(directories: String) -> Result<()> {
    let loader_context = || format!("Failed to initialize loader for: {}", directories);
    let loader = Loader::new(&directories).with_context(loader_context)?;

    let modules_context = || "Failed to retrieve modules".to_string();
    let modules = loader.get_modules().with_context(modules_context)?;

    println!(
        "\n{} {}\n",
        "::".bold().cyan(),
        "Module Registry".bold().cyan()
    );

    for module in modules {
        let name_padded = format!("{:<14}", module.manifest.module.name);
        let name_colored = name_padded.bold().green();
        let status_colored = Helper::format_status(&module);
        let version_colored = format!("v{:<7}", module.manifest.module.version).dimmed();
        let deps_colored = format!("deps:{:<22}", Helper::format_dependencies(&module)).dimmed();

        let extract_filename = |name: &std::ffi::OsStr| name.to_string_lossy().into_owned();
        let file_name = module
            .path
            .file_name()
            .map(extract_filename)
            .unwrap_or_default();
        let file_colored = file_name.dimmed();

        let source_managed = module.manifest.source.is_some();
        let managed_tag = if source_managed {
            " [src]".cyan().to_string()
        } else {
            String::new()
        };

        println!(
            "  {}  {}  {}  {}  {}{}",
            name_colored, status_colored, version_colored, deps_colored, file_colored, managed_tag
        );

        Helper::print_status_reason_detailed(&module.status);
    }

    println!();
    Ok(())
}

struct Helper;

impl Helper {
    fn format_dependencies(module: &DiscoveredModule) -> String {
        let dependencies = &module.manifest.module.deps;
        if dependencies.is_empty() {
            "[]".to_string()
        } else {
            let format_dependency = |dependency: &Dependency| match &dependency.version {
                Some(version) => format!("{}@{}", dependency.name, version),
                None => dependency.name.clone(),
            };
            let joined_dependencies = dependencies
                .iter()
                .map(format_dependency)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{}]", joined_dependencies)
        }
    }

    fn format_status(module: &DiscoveredModule) -> String {
        match &module.status {
            ModuleStatus::Loaded => match module.manifest.load_mode() {
                crate::core::manifest::LoadMode::Lazy => {
                    format!("{:<8}", "lazy").cyan().to_string()
                }
                crate::core::manifest::LoadMode::Event => {
                    format!("{:<8}", "event").blue().to_string()
                }
                crate::core::manifest::LoadMode::Eager => {
                    format!("{:<8}", "loaded").green().to_string()
                }
            },
            ModuleStatus::WarnDuplicateDep(_) => format!("{:<8}", "warn").yellow().to_string(),
            ModuleStatus::SkippedMissingCmd(_)
            | ModuleStatus::SkippedMissingAnyCmd(_)
            | ModuleStatus::SkippedMissingDep(_) => {
                format!("{:<8}", "skipped").yellow().to_string()
            }
            ModuleStatus::SkippedCycle(_) => format!("{:<8}", "cycle").red().to_string(),
            ModuleStatus::SkippedBadConstraint(_) | ModuleStatus::FailedManifest(_) => {
                format!("{:<8}", "error").red().to_string()
            }
        }
    }

    fn print_status_reason_detailed(status: &ModuleStatus) {
        match status {
            ModuleStatus::SkippedMissingCmd(command) => {
                println!(
                    "    {}",
                    format!("↳ missing required command: {}", command).yellow()
                );
            }
            ModuleStatus::SkippedMissingAnyCmd(commands) => {
                println!(
                    "    {}",
                    format!("↳ none of these commands found: {}", commands.join(", ")).yellow()
                );
            }
            ModuleStatus::SkippedMissingDep(dependency) => {
                println!(
                    "    {}",
                    format!("↳ missing or skipped dependency: {}", dependency).yellow()
                );
            }
            ModuleStatus::SkippedBadConstraint(detail) => {
                println!(
                    "    {}",
                    format!("↳ bad version constraint: {}", detail).red()
                );
            }
            ModuleStatus::SkippedCycle(path) => {
                println!(
                    "    {}",
                    format!("↳ circular dependency: {}", path.join(" → ")).red()
                );
            }
            ModuleStatus::FailedManifest(detail) => {
                println!("    {}", format!("↳ manifest error: {}", detail).red());
            }
            ModuleStatus::WarnDuplicateDep(dependency) => {
                println!(
                    "    {}",
                    format!("↳ duplicate dep entry in module.toml: '{}'", dependency).yellow()
                );
            }
            ModuleStatus::Loaded => {}
        }
    }
}
