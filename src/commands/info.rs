use crate::loader::types::ModuleStatus;
use crate::loader::Loader;
use anyhow::{bail, Result};
use colored::Colorize;
use std::path::PathBuf;

pub fn run(dir: PathBuf, module_name: String) -> Result<()> {
    let loader = Loader::new(dir);
    let modules = loader.get_modules()?;

    let target = modules.iter().find(|m| m.manifest.module.name == module_name);

    let m = match target {
        Some(m) => m,
        None => bail!("Module '{}' not found.", module_name),
    };

    println!("\n{} {}\n", "::".bold().cyan(), format!("Module: {}", m.manifest.module.name).bold().cyan());

    let status_text = match &m.status {
    ModuleStatus::Loaded => "loaded".green(),
    _ => "skipped".yellow(),
    };

    let file_name = m.path.file_name().unwrap().to_string_lossy();
    let desc = m.manifest.module.description.as_deref().unwrap_or("—");
    let deps = if m.manifest.module.deps.is_empty() { "—".to_string() } else { m.manifest.module.deps.join(", ") };
    let tags = if m.manifest.module.tags.is_empty() { "—".to_string() } else { m.manifest.module.tags.join(", ") };

    let kw = |s: &str| format!("{:<14}", s).bold().cyan();
    println!("  {} {}", kw("status"), status_text);
    println!("  {} {}", kw("file"), file_name.dimmed());
    println!("  {} {}", kw("path"), m.path.display().to_string().dimmed());
    println!("  {} {}", kw("desc"), desc);
    println!("  {} {}", kw("version"), m.manifest.module.version);
    println!("  {} {}", kw("deps"), deps);
    println!("  {} {}", kw("tags"), tags);

    // Show skip reason when relevant.
    match &m.status {
        ModuleStatus::SkippedMissingCmd(cmd) => {
            println!("  {} missing required command: {}", kw("skip reason"), cmd.yellow());
        }
        ModuleStatus::SkippedMissingAnyCmd(cmds) => {
            println!("  {} none of these commands found: {}", kw("skip reason"), cmds.join(", ").yellow());
        }
        ModuleStatus::SkippedMissingDep(dep) => {
            println!("  {} missing or skipped dependency: {}", kw("skip reason"), dep.yellow());
        }
        ModuleStatus::Loaded => {}
    }

    println!();

    let api = &m.manifest.api;
    let has_api = !api.functions.is_empty() || !api.variables.is_empty() || !api.aliases.is_empty();

    if has_api {
        println!("  {}", "Public API".bold().cyan());

        if !api.functions.is_empty() {
            println!("    {}", "functions:".dimmed());
            for f in &api.functions {
                println!("      {}", f.green());
            }
        }

        if !api.aliases.is_empty() {
            println!("    {}", "aliases:".dimmed());
            for (name, target) in &api.aliases {
                println!("      {:<14} {} {}", name.green(), "→".dimmed(), target.dimmed());
            }
        }

        if !api.variables.is_empty() {
            println!("    {}", "variables:".dimmed());
            for v in &api.variables {
                println!("      {}", v.yellow());
            }
        }
    } else {
        println!("  {}  {}", "Public API".bold().cyan(), "(none registered)".dimmed());
    }

    println!();
    Ok(())
}

