use crate::core::file_guard::TempFileGuard;
use crate::core::loader::Loader;
use crate::core::types::{DiscoveredModule, ModuleStatus};
use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::collections::HashSet;
use std::fs;
use std::process::Command;

pub fn run(directories: String) -> Result<()> {
    let loader_context = || format!("Failed to initialize loader for: {}", directories);
    let loader = Loader::new(&directories).with_context(loader_context)?;

    let modules_context = || "Failed to retrieve modules".to_string();
    let modules = loader.get_modules().with_context(modules_context)?;

    let filter_loaded =
        |discovered_module: &&DiscoveredModule| discovered_module.status == ModuleStatus::Loaded;
    let loaded_modules: Vec<_> = modules.iter().filter(filter_loaded).collect();

    if loaded_modules.is_empty() {
        println!("No loaded modules to profile.");
        return Ok(());
    }

    let script = Helper::generate_profile_script(&loaded_modules);

    let temporary_dir = std::env::temp_dir();
    let temporary_file_name = crate::core::common::temporary_name("profile");
    let temporary_path = temporary_dir.join(format!("{}.zsh", temporary_file_name));

    let mut guard = TempFileGuard::new(temporary_path.clone());

    let write_context = || "Failed to write temporary profiling script".to_string();
    fs::write(&temporary_path, &script).with_context(write_context)?;

    let execute_context = || "Failed to execute zsh".to_string();
    let output = Command::new("zsh")
        .arg("-f")
        .arg(&temporary_path)
        .output()
        .with_context(execute_context)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Profiling script failed:\n{}", stderr);
    }

    let remove_context = || "Failed to remove temporary profiling script".to_string();
    fs::remove_file(&temporary_path).with_context(remove_context)?;
    guard.defuse();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Helper::parse_profile_output(&stdout);

    let sort_predicate = |a: &(String, f64), b: &(String, f64)| {
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
    };
    results.sort_by(sort_predicate);

    let sum_ms = |(_, ms): &(String, f64)| *ms;
    let total: f64 = results.iter().map(sum_ms).sum();

    let fold_max = |accumulator: f64, (_, ms): &(String, f64)| f64::max(accumulator, *ms);
    let max_ms: f64 = results.iter().fold(0.0_f64, fold_max);

    let filter_deferred = |discovered_module: &&DiscoveredModule| {
        discovered_module.status == ModuleStatus::Loaded
            && discovered_module.manifest.api.defer_on_cmd
    };
    let extract_name =
        |discovered_module: &DiscoveredModule| discovered_module.manifest.module.name.clone();
    let deferred_modules: HashSet<String> = modules
        .iter()
        .filter(filter_deferred)
        .map(extract_name)
        .collect();

    Helper::print_profile_report(&results, &deferred_modules, total, max_ms);

    Ok(())
}

struct Helper;

impl Helper {
    fn generate_profile_script(loaded_modules: &[&DiscoveredModule]) -> String {
        let mut script = String::new();
        script.push_str("zmodload zsh/datetime\n");
        script.push_str("compdef() { : }\n");

        for module in loaded_modules {
            let init_path = module.path.join("init.zsh");
            if init_path.exists() {
                let escaped_path = init_path.to_string_lossy().replace('\'', "'\\''");
                script.push_str(&format!(
                    "t_start=$EPOCHREALTIME\n\
                     {{ source '{}'; }} >/dev/null 2>&1\n\
                     t_end=$EPOCHREALTIME\n\
                     echo \"{}: $(( (t_end - t_start) * 1000 ))\"\n",
                    escaped_path, module.manifest.module.name
                ));
            }
        }
        script
    }

    fn parse_profile_output(stdout: &str) -> Vec<(String, f64)> {
        let parse_line = |line: &str| {
            let (name, ms_str) = line.split_once(": ")?;
            let ms = ms_str.trim().parse::<f64>().ok()?;
            Some((name.to_string(), ms))
        };

        stdout.lines().filter_map(parse_line).collect()
    }

    fn print_profile_report(
        results: &[(String, f64)],
        deferred_modules: &HashSet<String>,
        total: f64,
        max_ms: f64,
    ) {
        println!(
            "\n{} {}\n",
            "::".bold().cyan(),
            "Module Load Profile".bold().cyan()
        );

        println!(
            "{}  {}  {}",
            format!("{:<20}", "Module").bold(),
            format!("{:>10}", "Time (ms)").bold(),
            "Relative".bold()
        );
        println!("{}", "─".repeat(52).dimmed());

        for (name, ms) in results {
            let deferred = deferred_modules.contains(name);
            let display_name = if deferred {
                format!("{} (def)", name)
            } else {
                name.clone()
            };
            let name_col = format!("{:<20}", display_name);
            let name_col = if deferred {
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
    }
}
