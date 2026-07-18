use crate::commands::install::head_commit;
use crate::core::types::DiscoveredModule;

use crate::core::loader::Loader;
use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use toml_edit::DocumentMut;

enum UpdateStatus {
    UpToDate,
    Updated,
    Failed(String),
}

struct GroupUpdateStatus {
    updated: usize,
    already_current: usize,
    failed: usize,
}

pub fn run(dirs: String, module_name: Option<String>) -> Result<()> {
    crate::core::common::require_git()?;

    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let managed: Vec<_> = modules
        .iter()
        .filter(|module| module.manifest.source.is_some())
        .filter(|module| match &module_name {
            Some(name) => &module.manifest.module.name == name,
            None => true,
        })
        .collect();

    if managed.is_empty() {
        if let Some(ref name) = module_name {
            bail!(
                "Module '{}' not found or has no [source] section.\n  \
                             Only modules installed via 'gai install' can be updated this way.",
                name
            );
        } else {
            println!(
                "No managed packages found.\n  \
                             Install packages with 'gai install <user/repo>' to track them here."
            );
            return Ok(());
        }
    }

    println!(
        "\n{} {}\n",
        "::".bold().cyan(),
        "Update Packages".bold().cyan()
    );

    let column_width = managed
        .iter()
        .map(|module| module.manifest.module.name.len())
        .max()
        .unwrap_or(12)
        .max(12);

    let mut updated = 0usize;
    let mut already_current = 0usize;
    let mut failed = 0usize;

    let (git_modules, collection_modules): (Vec<&&DiscoveredModule>, Vec<_>) = managed
        .iter()
        .partition(|module| module.path.join(".git").exists());

    for module in &git_modules {
        match Helper::update_module(module, column_width) {
            UpdateStatus::UpToDate => {
                println!("{}", "up to date".dimmed());
                already_current += 1;
            }
            UpdateStatus::Updated => {
                println!("{}", "updated".bold().green());
                updated += 1;
            }
            UpdateStatus::Failed(err) => {
                println!("{}", "failed".bold().red());
                eprintln!("    {}", err);
                failed += 1;
            }
        }
    }

    let mut by_url: HashMap<String, Vec<&DiscoveredModule>> = HashMap::new();
    for module in &collection_modules {
        let url = module.manifest.source.as_ref().unwrap().url.clone();
        by_url.entry(url).or_default().push(**module);
    }

    for (url, group) in &by_url {
        let group_stats = Helper::update_group(url, group, column_width)?;
        updated += group_stats.updated;
        already_current += group_stats.already_current;
        failed += group_stats.failed;
    }

    println!();
    let total = managed.len();
    let summary = format!(
        "{} checked  {} updated  {} current  {} failed",
        total,
        updated.to_string().bold(),
        already_current,
        if failed > 0 {
            failed.to_string().red().bold()
        } else {
            failed.to_string().normal()
        },
    );
    println!("  {}\n", summary.dimmed());

    if updated > 0 {
        println!(
            "{} Run {} to reload updated modules in your current session\n",
            "=>".bold().blue(),
            "gai reload".bold()
        );
    }

    if failed > 0 {
        bail!("{} package(s) failed to update", failed);
    }

    Ok(())
}

struct Helper;

impl Helper {
    fn update_module(module: &DiscoveredModule, column_width: usize) -> UpdateStatus {
        let name = &module.manifest.module.name;
        let source = match module.manifest.source.as_ref() {
            Some(source) => source,
            None => return UpdateStatus::Failed("No source definition".to_string()),
        };

        print!("  {:<width$}  ", name.green(), width = column_width);

        let path_string = module.path.to_string_lossy().into_owned();
        let mut arguments = vec!["-C".to_string(), path_string, "pull".to_string()];
        if let Some(ref b) = source.branch {
            arguments.push("origin".to_string());
            arguments.push(b.clone());
        }

        let output = Command::new("git")
            .args(&arguments)
            .env("LC_ALL", "C")
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("Already up to date") || stdout.contains("Already up-to-date") {
                    UpdateStatus::UpToDate
                } else {
                    if let Some(pin) = head_commit(&module.path) {
                        let toml_path = &module.path.join("module.toml");
                        if let Err(err) = Self::update_pin(toml_path, &pin) {
                            return UpdateStatus::Failed(format!("Failed to update pin: {}", err));
                        }
                    }
                    UpdateStatus::Updated
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                UpdateStatus::Failed(stderr.trim().to_string())
            }
            Err(err) => UpdateStatus::Failed(err.to_string()),
        }
    }

    fn update_group(
        url: &str,
        group: &[&DiscoveredModule],
        column_width: usize,
    ) -> Result<GroupUpdateStatus> {
        let mut stats = GroupUpdateStatus {
            updated: 0,
            already_current: 0,
            failed: 0,
        };

        let first_module = group[0];
        let source = first_module.manifest.source.as_ref().unwrap();
        let branch = source.branch.clone();
        let old_pin = source.pin.clone();

        let parent = first_module
            .path
            .parent()
            .context("Module has no parent directory")?;
        let temporary_directory = parent.join(crate::core::common::temporary_name("update"));

        if temporary_directory.exists() {
            let _ = fs::remove_dir_all(&temporary_directory).with_context(|| {
                format!(
                    "Failed to remove stale temporary directory: {}",
                    temporary_directory.display()
                )
            });
        }

        let mut arguments = vec!["clone".to_string()];
        if let Some(ref b) = branch {
            arguments.push("-b".to_string());
            arguments.push(b.clone());
        }

        let clone_status = Command::new("git")
            .args(&arguments)
            .env("LC_ALL", "C")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output();

        let (clone_ok, new_pin) = match clone_status {
            Ok(output) if output.status.success() => {
                let pin = head_commit(&temporary_directory);
                (true, pin)
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                for module in group {
                    let name = &module.manifest.module.name;
                    println!(
                        "  {:<width$}  {}",
                        name.green(),
                        "clone failed".bold().red(),
                        width = column_width
                    );
                    eprintln!("    {}", stderr.trim());
                    stats.failed += 1;
                }
                let _ = fs::remove_dir_all(&temporary_directory);
                return Ok(stats);
            }
            Err(err) => {
                for module in group {
                    let name = &module.manifest.module.name;
                    println!(
                        "  {:<width$}  {}",
                        name.green(),
                        "clone failed".bold().red(),
                        width = column_width
                    );
                    eprintln!("    {}", err);
                    stats.failed += 1;
                }
                let _ = fs::remove_dir_all(&temporary_directory);
                return Ok(stats);
            }
        };

        let already_up = old_pin.is_some() && new_pin.as_deref() == old_pin.as_deref();

        for module in group {
            let name = &module.manifest.module.name;
            print!("  {:<width$}  ", name.green(), width = column_width);

            if !clone_ok {
                continue;
            }

            if already_up {
                println!("{}", "up to date".dimmed());
                stats.already_current += 1;
                continue;
            }

            let subdirectory = Self::collection_subdirectory(&temporary_directory, name);

            match subdirectory {
                Some(path) => {
                    match Self::synchronise_modules(&path, &module.path, new_pin.as_deref(), module)
                    {
                        Ok(()) => {
                            println!("{}", "updated".bold().green());
                            stats.updated += 1;
                        }
                        Err(err) => {
                            println!("{}", "failed".bold().red());
                            eprintln!("    {}", err);
                            stats.failed += 1;
                        }
                    }
                }
                None => {
                    println!("{}", "not found in repo".bold().red());
                    eprintln!(
                        "    {} module '{}' has no matching subdirectory in {}",
                        "warn:".yellow(),
                        name,
                        url
                    );
                    stats.failed += 1;
                }
            }
        }

        let _ = fs::remove_dir_all(&temporary_directory);
        Ok(stats)
    }

    fn collection_subdirectory(clone_root: &Path, target_name: &str) -> Option<PathBuf> {
        let entries = fs::read_dir(clone_root).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let toml_path = path.join("module.toml");
            if !toml_path.exists() {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&toml_path)
                && let Ok(document) = content.parse::<DocumentMut>()
                    && document["module"]["name"].as_str() == Some(target_name) {
                        return Some(path);
                    }
        }
        None
    }

    fn synchronise_modules(
        source: &Path,
        destination: &Path,
        new_pin: Option<&str>,
        module: &DiscoveredModule,
    ) -> Result<()> {
        let entries = fs::read_dir(source)
            .with_context(|| format!("Failed to read {}", source.display()))?
            .flatten();

        for entry in entries {
            let source_path = entry.path();
            let destination_path = destination.join(entry.file_name());

            if source_path.is_dir() {
                fs::create_dir_all(&destination_path)?;
                Self::synchronise_modules(&source_path, &destination_path, None, module)?;
            } else {
                fs::copy(&source_path, &destination_path).with_context(|| {
                    format!(
                        "Failed to copy {} -> {}",
                        source_path.display(),
                        destination_path.display()
                    )
                })?;
            }
        }

        let toml_path = destination.join("module.toml");
        let content = fs::read_to_string(&toml_path)
            .with_context(|| format!("Failed to read {}", toml_path.display()))?;

        let source_entry = module.manifest.source.as_ref().unwrap();
        let updated = Self::rewrite_source(
            &content,
            &source_entry.url,
            source_entry.branch.as_deref(),
            new_pin,
        )
        .with_context(|| format!("Failed to write {}", toml_path.display()))?;

        fs::write(&toml_path, updated)
            .with_context(|| format!("Failed to write {}", toml_path.display()))?;

        Ok(())
    }

    fn rewrite_source(
        content: &str,
        url: &str,
        branch: Option<&str>,
        pin: Option<&str>,
    ) -> Result<String> {
        let mut document: DocumentMut = content
            .parse()
            .context("Failed to parse module.toml as TOML")?;

        let mut table = toml_edit::Table::new();
        table["url"] = toml_edit::value(url);

        if let Some(branch) = branch {
            table["branch"] = toml_edit::value(branch);
        }

        if let Some(pin) = pin {
            table["pin"] = toml_edit::value(pin);
        }

        document["source"] = toml_edit::Item::Table(table);
        Ok(document.to_string())
    }

    fn update_pin(path: &Path, new_pin: &str) -> Result<()> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let mut document: DocumentMut = content
            .parse()
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        if let Some(source) = document.get_mut("source")
            && let Some(table) = source.as_table_mut() {
                table["pin"] = toml_edit::value(new_pin);
            }

        fs::write(path, document.to_string())
            .with_context(|| format!("Failed to write {}", path.display()))?;

        Ok(())
    }
}
