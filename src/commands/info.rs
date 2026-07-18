use crate::core::loader::Loader;
use crate::core::manifest::Dependency;
use crate::core::types::{DiscoveredModule, ModuleStatus};
use anyhow::{Context, Result};
use colored::Colorize;

pub fn run(directories: String, module_name: String) -> Result<()> {
    let loader_context = || format!("Failed to initialize loader for: {}", directories);
    let loader = Loader::new(&directories).with_context(loader_context)?;

    let modules_context = || "Failed to retrieve modules".to_string();
    let modules = loader.get_modules().with_context(modules_context)?;

    let find_predicate = |discovered_module: &&DiscoveredModule| {
        discovered_module.manifest.module.name == module_name
    };
    let target_module = modules
        .iter()
        .find(find_predicate)
        .ok_or_else(|| anyhow::anyhow!("Module '{}' not found.", module_name))?;

    println!(
        "\n{} {}\n",
        "::".bold().cyan(),
        format!("Module: {}", target_module.manifest.module.name)
            .bold()
            .cyan()
    );

    Helper::print_general_metadata(target_module);
    Helper::print_status_reason(&target_module.status);
    println!();
    Helper::print_public_api(target_module);
    Helper::print_source_info(target_module);
    println!();

    Ok(())
}

struct Helper;

impl Helper {
    fn format_dependencies(module: &DiscoveredModule) -> String {
        let dependencies = &module.manifest.module.deps;
        if dependencies.is_empty() {
            "—".to_string()
        } else {
            let format_dependency = |dependency: &Dependency| match &dependency.version {
                Some(version) => format!("{}@{}", dependency.name, version),
                None => dependency.name.clone(),
            };
            dependencies
                .iter()
                .map(format_dependency)
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    fn print_general_metadata(module: &DiscoveredModule) {
        let status_text = match &module.status {
            ModuleStatus::Loaded => "loaded".green(),
            ModuleStatus::WarnDuplicateDep(_) => "warn".yellow(),
            ModuleStatus::SkippedBadConstraint(_) | ModuleStatus::FailedManifest(_) => {
                "error".red()
            }
            ModuleStatus::SkippedCycle(_) => "cycle".red(),
            _ => "skipped".yellow(),
        };

        let file_name = module
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        let description = module.manifest.module.description.as_deref().unwrap_or("—");
        let dependencies = Self::format_dependencies(module);

        let tags = if module.manifest.module.tags.is_empty() {
            "—".to_string()
        } else {
            module.manifest.module.tags.join(", ")
        };

        let keyword = |label: &str| format!("{:<14}", label).bold().cyan();
        println!("  {} {}", keyword("status"), status_text);
        println!("  {} {}", keyword("file"), file_name.dimmed());
        println!(
            "  {} {}",
            keyword("path"),
            module.path.display().to_string().dimmed()
        );
        println!("  {} {}", keyword("desc"), description);
        println!(
            "  {} {}",
            keyword("version"),
            module.manifest.module.version
        );
        println!("  {} {}", keyword("deps"), dependencies);
        println!("  {} {}", keyword("tags"), tags);

        let deferred = module.manifest.api.defer_on_cmd;
        let deferred_text = if deferred {
            "yes".cyan()
        } else {
            "no".dimmed()
        };
        println!("  {} {}", keyword("lazy"), deferred_text);
    }

    fn print_status_reason(status: &ModuleStatus) {
        let keyword = |label: &str| format!("{:<14}", label).bold().cyan();
        match status {
            ModuleStatus::SkippedMissingCmd(command) => {
                println!(
                    "  {} missing required command: {}",
                    keyword("skip reason"),
                    command.yellow()
                );
            }
            ModuleStatus::SkippedMissingAnyCmd(commands) => {
                println!(
                    "  {} none of these commands found: {}",
                    keyword("skip reason"),
                    commands.join(", ").yellow()
                );
            }
            ModuleStatus::SkippedMissingDep(dependency) => {
                println!(
                    "  {} missing or skipped dependency: {}",
                    keyword("skip reason"),
                    dependency.yellow()
                );
            }
            ModuleStatus::SkippedBadConstraint(detail) => {
                println!(
                    "  {} bad version constraint: {}",
                    keyword("error"),
                    detail.red()
                );
            }
            ModuleStatus::SkippedCycle(path) => {
                println!(
                    "  {} circular dependency: {}",
                    keyword("error"),
                    path.join(" → ").red()
                );
            }
            ModuleStatus::FailedManifest(detail) => {
                println!("  {} {}", keyword("error"), detail.red());
            }
            ModuleStatus::WarnDuplicateDep(dependency) => {
                println!(
                    "  {} duplicate dep entry in module.toml: '{}'",
                    keyword("warning"),
                    dependency.yellow()
                );
            }
            ModuleStatus::Loaded => {}
        }
    }

    fn print_public_api(module: &DiscoveredModule) {
        let api = &module.manifest.api;
        let registered_api =
            !api.functions.is_empty() || !api.variables.is_empty() || !api.aliases.is_empty();

        if registered_api {
            println!("  {}", "Public API".bold().cyan());

            if !api.functions.is_empty() {
                println!("    {}", "functions:".dimmed());
                for function in &api.functions {
                    println!("      {}", function.green());
                }
            }

            if !api.aliases.is_empty() {
                println!("    {}", "aliases:".dimmed());
                for (name, target) in &api.aliases {
                    println!(
                        "      {:<14} {} {}",
                        name.green(),
                        "→".dimmed(),
                        target.dimmed()
                    );
                }
            }

            if !api.variables.is_empty() {
                println!("    {}", "variables:".dimmed());
                for variable in &api.variables {
                    println!("      {}", variable.yellow());
                }
            }
        } else {
            println!(
                "  {}  {}",
                "Public API".bold().cyan(),
                "(none registered)".dimmed()
            );
        }
    }

    fn print_source_info(module: &DiscoveredModule) {
        if let Some(ref source) = module.manifest.source {
            println!();
            println!("  {}", "Source".bold().cyan());
            println!("    {:<10} {}", "url:".dimmed(), source.url.dimmed());
            if let Some(ref branch) = source.branch {
                println!("    {:<10} {}", "branch:".dimmed(), branch.dimmed());
            }
            if let Some(ref pin) = source.pin {
                println!("    {:<10} {}", "pin:".dimmed(), pin.dimmed());
            }
            println!(
                "    {:<10} {}",
                "update:".dimmed(),
                format!("gai update {}", module.manifest.module.name).green()
            );
        }
    }
}
