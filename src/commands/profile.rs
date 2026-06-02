use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::process::Command;
use crate::loader::Loader;
use crate::loader::types::ModuleStatus;

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

    // Generate the profiling script
    let mut script = String::new();
    script.push_str("zmodload zsh/datetime\n");
    for m in &loaded_modules {
        let init_path = m.path.join("init.zsh");
        if init_path.exists() {
            let escaped_path = init_path.to_string_lossy().replace('\'', "'\\''");
            script.push_str(&format!(
                "t_start=$EPOCHREALTIME\n\
                 source '{}'\n\
                 t_end=$EPOCHREALTIME\n\
                 echo \"{}: $(( (t_end - t_start) * 1000 ))\"\n",
                escaped_path, m.manifest.module.name
            ));
        }
    }

    // Write to a temporary file
    let temp_dir = std::env::temp_dir();
    let temp_file_path = temp_dir.join(format!("gaiety_profile_{}.zsh", std::process::id()));
    fs::write(&temp_file_path, script)
        .context("Failed to write temporary profiling script")?;

    // Run the script in zsh
    let output = Command::new("zsh")
        .arg("-f")
        .arg(&temp_file_path)
        .output()
        .context("Failed to execute zsh to run profiling script")?;

    // Clean up the temporary file
    let _ = fs::remove_file(&temp_file_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Profiling failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();

    for line in stdout.lines() {
        if let Some((name, ms_str)) = line.split_once(": ") {
            if let Ok(ms) = ms_str.trim().parse::<f64>() {
                results.push((name.to_string(), ms));
            }
        }
    }

    // Sort by duration descending
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let max_ms = results.iter().map(|(_, ms)| *ms).fold(0.0f64, f64::max);

    println!("\n{} {}\n", "::".bold().cyan(), "Module Load Profile".bold().cyan());
    println!("{:<24} {:>12}  {}", "Module".bold(), "Time (ms)".bold(), "Relative".bold());
    println!("{}", "-".repeat(56));

    let mut total = 0.0;
    for (name, ms) in &results {
        total += ms;
        let name_colored = name.bold().green();
        
        // Color-code based on performance thresholds
        let ms_colored = if *ms < 1.0 {
            format!("{:.3} ms", ms).green()
        } else if *ms < 5.0 {
            format!("{:.3} ms", ms).yellow()
        } else {
            format!("{:.3} ms", ms).red().bold()
        };

        // Generate a relative bar chart
        let bar_width = if max_ms > 0.0 {
            ((*ms / max_ms) * 16.0).round() as usize
        } else {
            0
        };
        let bar = "█".repeat(bar_width).cyan();

        println!("{:<24} {:>12}  {}", name_colored, ms_colored, bar);
    }
    println!("{}", "-".repeat(56));
    println!("{:<24} {:>12}", "Total".bold(), format!("{:.3} ms", total).bold().cyan());

    Ok(())
}
