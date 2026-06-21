use crate::core::Loader;
use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::io::{self, Write};

pub fn run(dirs: String) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let mut to_prune = Vec::new();
    let mut pruned_set = std::collections::HashSet::new();

    loop {
        let mut in_degrees: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for m in &modules {
            if pruned_set.contains(&m.manifest.module.name) {
                continue;
            }
            in_degrees
                .entry(m.manifest.module.name.clone())
                .or_insert(0);
            for dep in &m.manifest.module.deps {
                if !pruned_set.contains(&dep.name) {
                    *in_degrees.entry(dep.name.clone()).or_insert(0) += 1;
                }
            }
        }

        let mut found_any = false;
        for m in &modules {
            let name = &m.manifest.module.name;
            if pruned_set.contains(name) {
                continue;
            }
            if m.manifest.module.implicit == Some(true)
                && let Some(&deg) = in_degrees.get(name)
                && deg == 0
            {
                to_prune.push(m.clone());
                pruned_set.insert(name.clone());
                found_any = true;
            }
        }

        if !found_any {
            break;
        }
    }

    if to_prune.is_empty() {
        println!("\nNo orphaned dependencies to prune.\n");
        return Ok(());
    }

    println!(
        "\n{} {}\n",
        "::".bold().cyan(),
        "Prune Orphaned Dependencies".bold().cyan()
    );
    for m in &to_prune {
        println!(
            "  {:<14} {}",
            m.manifest.module.name.red(),
            m.path.display().to_string().dimmed()
        );
    }
    println!();

    print!(
        "{} Remove {} orphaned module(s)? [y/N] ",
        "?".bold().yellow(),
        to_prune.len()
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().eq_ignore_ascii_case("y") {
        let mut affected_dirs = std::collections::HashSet::new();
        for m in &to_prune {
            if let Some(parent) = m.path.parent() {
                affected_dirs.insert(parent.to_path_buf());
            }
            fs::remove_dir_all(&m.path)?;
        }
        for dir in affected_dirs {
            super::rm::renumber_modules(&dir)?;
        }
        println!(
            "{} pruned {} module(s)\n",
            "✓".bold().green(),
            to_prune.len()
        );
    } else {
        println!("{} aborted\n", "!".bold().yellow());
    }

    Ok(())
}
