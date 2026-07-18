use crate::core::loader::Loader;
use crate::core::types::DiscoveredModule;
use anyhow::{Context, Result, anyhow, bail};
use colored::Colorize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{fs, process::Command};
use toml_edit::DocumentMut;

struct ParsedSpec {
    url: String,
    repository: String,
    branch: Option<String>,
}

impl ParsedSpec {
    pub fn new(spec: &str, branch_override: Option<String>) -> Result<ParsedSpec> {
        let (base, inline_branch) = match spec.rfind('@') {
            Some(idx) if idx > spec.rfind('/').unwrap_or(0) => {
                (&spec[..idx], Some(spec[idx + 1..].to_string()))
            }
            _ => (spec, None),
        };

        let branch = branch_override.or(inline_branch);
        let is_url = base.starts_with("http://") || base.starts_with("https://");

        let raw_url = if is_url {
            base.to_string()
        } else if let Some(path) = base.strip_prefix("github:") {
            format!("https://github.com/{}", path)
        } else if let Some(path) = base.strip_prefix("gitlab:") {
            format!("https://gitlab.com/{}", path)
        } else if base.contains('/') {
            format!("https://github.com/{}", base)
        } else {
            anyhow::bail!(
                "Cannot parse package spec '{}'.\n  \
                 Expected one of:\n  \
                 • user/repo\n  \
                 • user/repo@branch\n  \
                 • github:user/repo\n  \
                 • gitlab:user/repo\n  \
                 • https://host/user/repo.git",
                spec
            );
        };

        let sanitized_base = raw_url.trim_end_matches('/').trim_end_matches(".git");
        let repository = sanitized_base
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow!("Could not extract repository name from '{}'", spec))?
            .to_string();

        let url = format!("{}.git", sanitized_base);

        Ok(Self {
            url,
            repository,
            branch,
        })
    }
}

pub fn repo_to_module(repo_name: &str) -> String {
    let mut string: String = repo_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();

    if string.starts_with(|character: char| character.is_ascii_digit()) {
        string.insert(0, '_');
    }
    string
}

pub fn name_valid(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().map_or(false, crate::core::common::valid_fn)
        && chars.all(crate::core::common::valid_fn)
}

pub fn head_commit(directory: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(["rev-parse", "--short", "HEAD"])
        .env("LC_ALL", "C")
        .output()
        .ok()?;

    if output.status.success() {
        let sha = String::from_utf8(output.stdout).ok()?;
        let sha = sha.trim();
        if !sha.is_empty() {
            return Some(sha.to_string());
        }
    }
    None
}

pub fn install_recursive(
    dirs: &str,
    spec: &str,
    name_override: Option<String>,
    branch_override: Option<String>,
    target: Option<PathBuf>,
    visited: &mut HashSet<String>,
) -> Result<()> {
    if !visited.insert(spec.to_string()) {
        bail!("Circular remote dependency detected for the spec: {}", spec);
    }
    crate::core::common::require_git()?;

    let parsed = ParsedSpec::new(spec, branch_override)?;
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let write_directory = target
        .clone()
        .unwrap_or_else(|| loader.default_write().clone());

    let temporary_name = crate::core::common::temporary_name("install");
    let temporary_directory = write_directory.join(&temporary_name);

    if temporary_directory.exists() {
        fs::remove_dir_all(&temporary_directory).with_context(|| {
            format!(
                "Failed to remove stale temporary directory: {}",
                temporary_directory.display()
            )
        })?;
    }

    println!(
        "\n{} {}\n",
        "::".bold().cyan(),
        "Install Package".bold().cyan()
    );

    println!("  {:<12} {}", "source:".dimmed(), parsed.url.dimmed());
    if let Some(ref b) = parsed.branch {
        println!("  {:<12} {}", "branch:".dimmed(), b.dimmed());
    }

    Helper::clone_repository(&parsed, &temporary_directory)?;

    let pin = head_commit(&temporary_directory);
    let source_block = Helper::build_source(&parsed, pin.as_deref());

    let collection_directories = Helper::detect_collection(&temporary_directory);
    let is_single = temporary_directory.join("module.toml").exists();

    todo!()
}

struct Helper;

impl Helper {
    const DOT_ZSH: &str = ".zsh";
    const DOT_PLUGIN_ZSH: &str = ".plugin.zsh";
    const ZSH_THEME: &str = ".zsh-theme";
    const SEARCH_SUFFIXES: [&str; 3] = [Self::DOT_ZSH, Self::DOT_PLUGIN_ZSH, Self::ZSH_THEME];

    fn clone_repository(parsed: &ParsedSpec, temporary_directory: &Path) -> Result<()> {
        let mut arguments = vec!["clone".to_string()];
        if let Some(branch) = parsed.branch {
            arguments.push("-b".to_string());
            arguments.push(branch.clone());
        }
        arguments.push(parsed.url.clone());
        arguments.push(temporary_directory.to_string_lossy().into_owned());

        let clone_status = Command::new("git")
            .args(&arguments)
            .env("LC_ALL", "C")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .status();

        match clone_status {
            Ok(status) if status.success() => Ok(()),
            _ => {
                if temporary_directory.exists() {
                    let _ = fs::remove_dir_all(temporary_directory);
                }
                bail!("git clone failed");
            }
        }
    }

    fn install_single(
        temporary_directory: &Path,
        parsed: &ParsedSpec,
        source_block: &str,
        modules: &[DiscoveredModule],
        directory: &Path,
        loader: &Loader,
        name_override: Option<String>,
    ) -> Result<()> {
        let module_name = name_override.unwrap_or_else(|| repo_to_module(&parsed.repository));

        if !name_valid(&module_name) {
            bail!(
                "Derived module name '{}' is not valid (must match [a-zA-Z_][a-zA-Z0-9_]*).\n  Use --name to provide a valid name.",
                module_name
            );
        }

        if modules
            .iter()
            .any(|module| module.manifest.module.name == module_name)
        {
            bail!(
                "Module '{}' already exists. Use 'gai update {}' to pull the latest version.",
                module_name,
                module_name
            );
        }

        todo!()
    }

    fn install_missing(
        dirs: &str,
        path: Option<PathBuf>,
        visited: &mut HashSet<String>,
    ) -> Result<()> {
        let loader = Loader::new(dirs)?;
        let updated = loader.get_modules()?;

        let loaded: HashSet<&str> = updated
            .iter()
            .map(|module| module.manifest.module.name.as_str())
            .collect();

        for module in &updated {
            for depency in &*module.manifest.module.deps {
                if let Some(ref dep_source) = depency.source {
                    if !loaded.contains(depency.name.as_str()) {
                        println!(
                            "{} Installing missing dependency '{}' from '{}'...",
                            "->".bold().blue(),
                            depency.name.green(),
                            dep_source.dimmed()
                        );
                    }
                }
            }
        }

        todo!()
    }

    fn detect_collection(directory: &Path) -> Vec<PathBuf> {
        let valid_module = |path: &PathBuf| -> bool {
            path.is_dir() && path.join("module.toml").exists() && path.join("init.zsh").exists()
        };
        let mut directories: Vec<PathBuf> = fs::read_dir(directory)
            .into_iter()
            .flatten()
            .filter_map(|element| element.ok())
            .map(|element| element.path())
            .filter(valid_module)
            .collect();
        directories.sort();
        directories
    }

    fn next_prefix(
        modules: &[DiscoveredModule],
        write_index: Option<usize>,
        directory: &Path,
    ) -> u32 {
        modules
            .iter()
            .filter(|module| match write_index {
                Some(index) => module.dir_index == index,
                None => module.path.parent() == Some(directory),
            })
            .filter_map(|module| module.prefix_order)
            .max()
            .unwrap_or(0)
            + 1
    }

    fn copy_dir(source: &Path, destination: &Path) -> Result<()> {
        fs::create_dir_all(destination)
            .with_context(|| format!("Failed to create directory: {}", destination.display()))?;

        let elements = fs::read_dir(source)
            .with_context(|| format!("Failed to read directory: {}", source.display()))?;

        for element in elements {
            let element = element.with_context(|| {
                format!("Failed to process directory entry in: {}", source.display())
            })?;

            let source_path = element.path();
            let file_name = element.file_name();
            let destination_path = destination.join(file_name);

            if source_path.is_dir() {
                Self::copy_dir(&source_path, &destination_path)?;
            } else {
                fs::copy(&source_path, &destination_path).with_context(|| {
                    format!(
                        "Failed to copy file from {} to {}",
                        source_path.display(),
                        destination_path.display()
                    )
                })?;
            }
        }
        Ok(())
    }

    fn build_source(parsed: &ParsedSpec, pin: Option<&str>) -> String {
        use std::fmt::Write as _;
        let mut string = String::from("[source]\n");
        let _ = writeln!(string, "url    = \"{}\"", parsed.url);
        if let Some(ref b) = parsed.branch {
            let _ = writeln!(string, "branch = \"{}\"", b);
        }
        if let Some(p) = pin {
            let _ = writeln!(string, "pin    = \"{}\"", p);
        }
        string
    }

    fn toml_name(path: &Path) -> Result<String> {
        let content = fs::read_to_string(path)?;
        let document: DocumentMut = content.parse().context("Failed to parse TOML")?;
        document["module"]["name"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow!("name field not found"))
    }

    fn detect_main(directory: &Path, repository: &str) -> Option<String> {
        for suffix in Self::SEARCH_SUFFIXES {
            let candidate = format!("{}{}", repository, suffix);
            if directory.join(&candidate).exists() {
                return Some(candidate);
            }
        }

        let mut plugin_files = Self::read_filenames(directory)
            .filter(|element| element.ends_with(Self::DOT_PLUGIN_ZSH));

        if let Some(first) = plugin_files.next() {
            if plugin_files.next().is_none() {
                return Some(first);
            }
        }

        let mut zsh_files = Self::read_filenames(directory)
            .filter(|element| element.ends_with(Self::DOT_ZSH) && element != "init.zsh");

        if let Some(first) = zsh_files.next() {
            if zsh_files.next().is_none() {
                return Some(first);
            }
        }

        None
    }

    fn read_filenames(directory: &Path) -> impl Iterator<Item = String> {
        fs::read_dir(directory)
            .into_iter()
            .flatten()
            .filter_map(|element| element.ok())
            .filter(|element| element.path().is_file())
            .filter_map(|element| element.file_name().into_string().ok())
    }
}
