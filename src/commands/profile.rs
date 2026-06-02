use crate::loader::types::ModuleStatus;
use crate::loader::Loader;
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::process::Command;

pub fn run(dirs: String) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let loaded_modules: Vec<_> = modules
        .iter()
        .filter(|m| m.status == ModuleStatus::Loaded)
        .collect();

    if loaded_modules.is_empty() {
        println!("No loaded modules to profile.");
        return Ok(());
    }

    let mut script = String::new();
    script.push_str("zmodload zsh/datetime\n"); 
    script.push_str("compdef() { : }\n");

    for m in &loaded_modules {
        let init_path = m.path.join("init.zsh");
        if init_path.exists() {
            let escaped_path = init_path.to_string_lossy().replace('\'', "'\\''");
            script.push_str(&format!(
                "t_start=$EPOCHREALTIME\n\
                 {{ source '{}'; }} >/dev/null 2>&1\n\
                 t_end=$EPOCHREALTIME\n\
                 echo \"{}: $(( (t_end - t_start) * 1000 ))\"\n",
                escaped_path, m.manifest.module.name
            ));
        }
    }

    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("gaiety_profile_{}.zsh", std::process::id()));

    fs::write(&temp_path, &script)
        .context("Failed to write temporary profiling script")?;

    let output = Command::new("zsh")
        .arg("-f")
        .arg(&temp_path)
        .output()
        .context("Failed to execute zsh")?;

    let _ = fs::remove_file(&temp_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Profiling script failed:\n{}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut results: Vec<(String, f64)> = stdout
        .lines()
        .filter_map(|line| {
            let (name, ms_str) = line.split_once(": ")?;
            let ms = ms_str.trim().parse::<f64>().ok()?;
            Some((name.to_string(), ms))
        })
        .collect();

    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let total: f64 = results.iter().map(|(_, ms)| ms).sum();
    let max_ms: f64 = results.iter().map(|(_, ms)| *ms).fold(0.0_f64, f64::max);

    let deferred_modules: std::collections::HashSet<String> = modules
        .iter()
        .filter(|m| m.status == ModuleStatus::Loaded && m.manifest.api.defer_on_cmd)
        .map(|m| m.manifest.module.name.clone())
        .collect();

    println!("\n{} {}\n", "::".bold().cyan(), "Module Load Profile".bold().cyan());

    println!(
        "{}  {}  {}",
        format!("{:<20}", "Module").bold(),
        format!("{:>10}", "Time (ms)").bold(),
        "Relative".bold()
    );
    println!("{}", "─".repeat(52).dimmed());

    for (name, ms) in &results {
        let is_deferred = deferred_modules.contains(name);
        let display_name = if is_deferred {
            format!("{} (def)", name)
        } else {
            name.clone()
        };
        let name_col = format!("{:<20}", display_name);
        let name_col = if is_deferred {
            name_col.bold().blue()
        } else {
            name_col.bold().green()
        };

        let ms_str = format!("{:.3} ms", ms);
        let ms_col = format!("{:>10}", ms_str);
        let ms_col = if *ms < 1.0 {
            ms_col.green()
        } else if *ms < 5.0 {
            ms_col.yellow()
        } else {
            ms_col.red().bold()
        };

        let bar_width = if max_ms > 0.0 {
            ((*ms / max_ms) * 20.0).round() as usize
        } else {
            0
        };
        let bar = "█".repeat(bar_width).cyan();

        println!("{}  {}  {}", name_col, ms_col, bar);
    }

    println!("{}", "─".repeat(52).dimmed());
    println!(
        "{}  {}",
        format!("{:<20}", "Total").bold(),
        format!("{:>10}", format!("{:.3} ms", total)).bold().cyan()
    );
    println!();

    Ok(())
}
