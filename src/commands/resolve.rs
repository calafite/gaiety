use crate::core::Loader;
use crate::core::types::ModuleStatus;
use crate::commands::install::install_recursive;
use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;

pub fn run(dirs: String) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    println!("\n{} {}\n", "::".bold().cyan(), "Resolving Dependencies".bold().cyan());

    let mut resolved_any = false;
    let mut visited = HashSet::new();

    for m in &modules {
        match &m.status {
            ModuleStatus::SkippedMissingDep(dep_name) | ModuleStatus::SkippedBadConstraint(dep_name) => {
                // Find if the dependent module specified a source for this dependency
                if let Some(dep) = m.manifest.module.deps.iter().find(|d| &d.name == dep_name) {
                    if let Some(ref source) = dep.source {
                        println!(
                            "Found remote source for missing dependency '{}' required by '{}': {}",
                            dep_name.green(),
                            m.manifest.module.name.yellow(),
                            source.dimmed()
                        );
                        install_recursive(&dirs, source, Some(dep_name.clone()), None, None, &mut visited)?;
                        resolved_any = true;
                    }
                }
            }
            _ => {}
        }
    }

    if resolved_any {
        println!("\n{} Dependencies resolved successfully.\n", "✓".bold().green());
    } else {
        println!("No missing remote dependencies found to resolve.\n");
    }

    Ok(())
}
