use crate::core::loader::Loader;
use crate::core::types::ModuleStatus;
use crate::validator::commands::CommandValidator;
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

pub fn default_cache_path() -> PathBuf {
    let path = std::env::var("GAI_CACHE")
        .ok()
        .filter(|string| !string.is_empty());

    if let Some(path) = path {
        return PathBuf::from(path);
    }

    let base = std::env::var("XDG_CACHE_HOME")
        .ok()
        .filter(|string| !string.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(shellexpand::tilde("~/.cache").as_ref()));

    base.join("gaiety").join("init.zsh")
}

pub fn run(dirs: String, output: Option<PathBuf>) -> Result<()> {
    let cache_path = output.unwrap_or_else(default_cache_path);

    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    for warning in CommandValidator::comps(&modules) {
        eprintln!("{} {}", "warn:".bold().yellow(), warning);
    }

    let zsh_code = loader.generate_init(&modules)?;

    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create cache directory: {}", parent.display()))?;
    }

    fs::write(&cache_path, &zsh_code)
        .with_context(|| format!("Failed to write cache: {}", cache_path.display()))?;

    zcompile_parallel(&modules, &cache_path)?;

    let loaded = modules
        .iter()
        .filter(|module| module.status == ModuleStatus::Loaded)
        .count();

    let warned = modules
        .iter()
        .filter(|module| matches!(module.status, ModuleStatus::WarnDuplicateDep(_)))
        .count();

    let skipped = modules.len() - loaded - warned;

    let warn_note = if warned > 0 {
        format!(", {} with warnings", warned)
    } else {
        String::new()
    };
    let skip_note = if skipped > 0 {
        format!(", {} skipped", skipped)
    } else {
        String::new()
    };

    eprintln!(
        "{} synced {} module{}{}{} → {}",
        "✓".bold().green(),
        loaded,
        if loaded == 1 { "" } else { "s" },
        warn_note,
        skip_note,
        cache_path.display().to_string().dimmed()
    );

    Ok(())
}

fn zcompile_parallel(
    modules: &[crate::core::types::DiscoveredModule],
    cache_path: &Path,
) -> Result<()> {
    let mut script = String::new();

    for module in modules {
        if module.status != ModuleStatus::Loaded {
            continue;
        }
        let init_path = module.path.join("init.zsh");
        if init_path.exists() {
            script.push_str(&format!(
                "zcompile -- '{}' 2>/dev/null &\n",
                sq_escape(&init_path.to_string_lossy())
            ));
        }
    }

    script.push_str(&format!(
        "zcompile -- '{}' 2>/dev/null &\n",
        sq_escape(&cache_path.to_string_lossy())
    ));
    script.push_str("wait\n");

    let temp_path = std::env::temp_dir().join(
        //
        format!(
            //
            "gaiety_zcompile_{}.zsh",
            std::process::id()
        ),
    );

    fs::write(&temp_path, &script).with_context(|| {
        format!(
            "Failed to write temporary zcompile script: {}",
            temp_path.display()
        )
    })?;

    let run_compilation = || -> Result<()> {
        let status = std::process::Command::new("zsh")
            .arg("-f")
            .arg(&temp_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .with_context(|| "Failed to execute zsh for zcompilation")?;

        if !status.success() {
            eprintln!(
                "{} zcompile failed with status: {}",
                "warn:".bold().yellow(),
                status
            );
        }
        Ok(())
    };

    let compile_result = run_compilation();

    let cleanup_result = fs::remove_file(&temp_path).with_context(|| {
        format!(
            "Failed to remove temporary zcompile script: {}",
            temp_path.display()
        )
    });

    compile_result.and(cleanup_result)
}

fn sq_escape(string: &str) -> String {
    string.replace('\'', r"'\''")
}
