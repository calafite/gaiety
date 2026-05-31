use crate::loader::types::ModuleStatus;
use crate::loader::Loader;
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
            ModuleStatus::Loaded => format!("{:<8}", "loaded").green(),
            ModuleStatus::SkippedMissingCmd(_)
            | ModuleStatus::SkippedMissingAnyCmd(_)
            | ModuleStatus::SkippedMissingDep(_) => format!("{:<8}", "skipped").yellow(),
        };

        let version_colored = format!("v{:<7}", m.manifest.module.version).dimmed();

        let deps = if m.manifest.module.deps.is_empty() {
            "[]".to_string()
        } else {
            format!("[{}]", m.manifest.module.deps.iter()
            .map(|d| match &d.version {
                Some(v) => format!("{}@{}", d.name, v),
                None => d.name.clone(),
            })
            .collect::<Vec<_>>()
            .join(", "))
        };

        let deps_colored = format!("deps:{:<22}", deps).dimmed();
        let file_colored = m.path.file_name().unwrap().to_string_lossy().dimmed();

        println!(
            "  {}  {}  {}  {}  {}",
            name_colored, status_colored, version_colored, deps_colored, file_colored
        );

        match &m.status {
            ModuleStatus::SkippedMissingCmd(cmd) => {
                let msg = format!("↳ missing required command: {}", cmd);
                println!("    {}", msg.yellow());
            }
            ModuleStatus::SkippedMissingAnyCmd(cmds) => {
                let msg = format!("↳ none of these commands found: {}", cmds.join(", "));
                println!("    {}", msg.yellow());
            }
            ModuleStatus::SkippedMissingDep(dep) => {
                let msg = format!("↳ missing or skipped dependency: {}", dep);
                println!("    {}", msg.yellow());
            }
            ModuleStatus::Loaded => {}
        }
    }
    println!();
    Ok(())
}

