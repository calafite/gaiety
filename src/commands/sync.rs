use crate::loader::types::ModuleStatus;
use crate::loader::Loader;
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

pub fn default_cache_path() -> PathBuf {
    if let Ok(p) = std::env::var("GAI_CACHE") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }

    let base = std::env::var("XDG_CACHE_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(shellexpand::tilde("~/.cache").as_ref()));

    base.join("gaiety").join("init.zsh")
}

pub fn run(dirs: String, output: Option<PathBuf>) -> Result<()> {
    let cache_path = output.unwrap_or_else(default_cache_path);

    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    for warning in loader.check_completions(&modules) {
        eprintln!("{} {}", "warn:".bold().yellow(), warning);
    }

    let zsh_code = loader.generate_init_from(&modules)?;

    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create cache directory: {}", parent.display()))?;
    }

    fs::write(&cache_path, &zsh_code)
        .with_context(|| format!("Failed to write cache: {}", cache_path.display()))?;

    let loaded = modules
        .iter()
        .filter(|m| m.status == ModuleStatus::Loaded)
        .count();

    let skipped = modules.len() - loaded;
    let skip_note = if skipped > 0 {
        format!(", {} skipped", skipped)
    } else {
        String::new()
    };

    eprintln!(
        "{} synced {} module{}{} → {}",
        "✓".bold().green(),
        loaded,
        if loaded == 1 { "" } else { "s" },
        skip_note,
        cache_path.display().to_string().dimmed()
    );

    Ok(())
}
