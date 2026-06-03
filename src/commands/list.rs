use crate::core::types::ModuleStatus;
use crate::core::Loader;
use anyhow::Result;
use colored::Colorize;

pub fn run(dirs: String) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    println!("\n{} {}\n", "::".bold().cyan(), "Module Registry".bold().cyan());

    for m in modules {
        let name_padded = format!("{:<14}", m.manifest.module.name);
        let name_colored = name_padded.bold().green();

        let status_colored = match &m.status {
            ModuleStatus::Loaded => {
                if m.manifest.api.defer_on_cmd {
                    format!("{:<8}", "lazy").cyan()
                } else {
                    format!("{:<8}", "loaded").green()
                }
            }
            ModuleStatus::WarnDuplicateDep(_) => format!("{:<8}", "warn").yellow(),
            ModuleStatus::SkippedMissingCmd(_)
            | ModuleStatus::SkippedMissingAnyCmd(_)
            | ModuleStatus::SkippedMissingDep(_) => format!("{:<8}", "skipped").yellow(),
            ModuleStatus::SkippedCycle(_) => format!("{:<8}", "cycle").red(),
            ModuleStatus::SkippedBadConstraint(_)
            | ModuleStatus::FailedManifest(_) => format!("{:<8}", "error").red(),
        };

        let version_colored = format!("v{:<7}", m.manifest.module.version).dimmed();

        let deps = if m.manifest.module.deps.is_empty() {
            "[]".to_string()
        } else {
            format!(
                "[{}]",
                m.manifest.module.deps
                    .iter()
                    .map(|d| match &d.version {
                        Some(v) => format!("{}@{}", d.name, v),
                        None => d.name.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        let deps_colored = format!("deps:{:<22}", deps).dimmed();
        let file_colored = m.path.file_name().unwrap().to_string_lossy().dimmed();

        let managed_tag = if m.manifest.source.is_some() {
            " [src]".cyan().to_string()
        } else {
            String::new()
        };

        println!(
            "  {}  {}  {}  {}  {}{}",
            name_colored, status_colored, version_colored, deps_colored, file_colored, managed_tag
        );

        match &m.status {
            ModuleStatus::SkippedMissingCmd(cmd) => {
                println!("    {}", format!("↳ missing required command: {}", cmd).yellow());
            }
            ModuleStatus::SkippedMissingAnyCmd(cmds) => {
                println!(
                    "    {}",
                    format!("↳ none of these commands found: {}", cmds.join(", ")).yellow()
                );
            }
            ModuleStatus::SkippedMissingDep(dep) => {
                println!(
                    "    {}",
                    format!("↳ missing or skipped dependency: {}", dep).yellow()
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
                println!(
                    "    {}",
                    format!("↳ manifest error: {}", detail).red()
                );
            }
            ModuleStatus::WarnDuplicateDep(dep) => {
                println!(
                    "    {}",
                    format!("↳ duplicate dep entry in module.toml: '{}'", dep).yellow()
                );
            }
            ModuleStatus::Loaded => {}
        }
    }
    println!();
    Ok(())
}
