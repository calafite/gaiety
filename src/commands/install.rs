use crate::core::loader::Loader;
use crate::core::types::DiscoveredModule;
use anyhow::{Context, Result, anyhow, bail};
use colored::Colorize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
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
            Some(index) if index > spec.rfind('/').unwrap_or(0) => {
                (&spec[..index], Some(spec[index + 1..].to_string()))
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

pub fn run(
    dirs: String,
    spec: String,
    name_override: Option<String>,
    branch_override: Option<String>,
    target: Option<PathBuf>,
) -> Result<()> {
    let mut visited = HashSet::new();
    install_recursive(
        &dirs,
        &spec,
        name_override,
        branch_override,
        target,
        &mut visited,
    )
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
    let loader = Loader::new(dirs)?;
    let modules = loader.get_modules()?;

    let get_default_write = || loader.default_write().clone();
    let write_directory = target.clone().unwrap_or_else(get_default_write);

    let temporary_name = crate::core::common::temporary_name("install");
    let temporary_directory = write_directory.join(&temporary_name);

    if temporary_directory.exists() {
        let remove_error_context = || {
            format!(
                "Failed to remove stale temporary directory: {}",
                temporary_directory.display()
            )
        };
        fs::remove_dir_all(&temporary_directory).with_context(remove_error_context)?;
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

    let result = if is_single || collection_directories.is_empty() {
        Helper::install_single(
            &temporary_directory,
            &parsed,
            &source_block,
            &modules,
            &write_directory,
            &loader,
            name_override,
        )
    } else {
        if name_override.is_some() {
            eprintln!(
                "{} --name is ignored for collection repositories; module names come from each module.toml",
                "warn:".bold().yellow()
            );
        }
        Helper::install_collection(
            &collection_directories,
            &parsed,
            &source_block,
            &modules,
            &write_directory,
            &loader,
        )
    };

    let _ = fs::remove_dir_all(&temporary_directory);
    result?;

    Helper::install_missing(dirs, target, visited)?;

    Ok(())
}

struct Helper;

impl Helper {
    const DOT_ZSH: &str = ".zsh";
    const DOT_PLUGIN_ZSH: &str = ".plugin.zsh";
    const ZSH_THEME: &str = ".zsh-theme";
    const SEARCH_SUFFIXES: [&str; 3] = [Self::DOT_ZSH, Self::DOT_PLUGIN_ZSH, Self::ZSH_THEME];

    fn clone_repository(parsed: &ParsedSpec, temporary_directory: &Path) -> Result<()> {
        let mut arguments = vec!["clone".to_string()];
        if let Some(branch) = &parsed.branch {
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
        let fallback_module_name = || repo_to_module(&parsed.repository);
        let module_name = name_override.unwrap_or_else(fallback_module_name);

        if !name_valid(&module_name) {
            bail!(
                "Derived module name '{}' is not valid (must match [a-zA-Z_][a-zA-Z0-9_]*).\n  Use --name to provide a valid name.",
                module_name
            );
        }

        let matches_module_name = |discovered_module: &DiscoveredModule| {
            discovered_module.manifest.module.name == module_name
        };
        let name_conflict = modules.iter().any(matches_module_name);

        if name_conflict {
            bail!(
                "Module '{}' already exists. Use 'gai update {}' to pull the latest version.",
                module_name,
                module_name
            );
        }

        let matches_directory = |captured_directory: &PathBuf| captured_directory == directory;
        let write_index = loader.dirs.iter().position(matches_directory);

        let max_prefix = Helper::next_prefix(modules, write_index, directory);
        let directory_name = format!("{:02}_{}", max_prefix, module_name);
        let module_directory = directory.join(&directory_name);

        if module_directory.exists() {
            bail!("Directory already exists: {}", module_directory.display());
        }

        println!("  {:<12} {}", "module:".dimmed(), module_name.green());
        println!(
            "  {:<12} {}\n",
            "dest:".dimmed(),
            module_directory.display().to_string().dimmed()
        );

        Helper::copy_dir(temporary_directory, &module_directory)?;

        let has_toml = module_directory.join("module.toml").exists();

        let fallback_to_module_name = |_error: anyhow::Error| module_name.clone();
        let final_name = if has_toml {
            let target_toml = &module_directory.join("module.toml");
            Helper::toml_name(target_toml).unwrap_or_else(fallback_to_module_name)
        } else {
            module_name.clone()
        };

        if has_toml {
            let toml_path = module_directory.join("module.toml");
            let existing = fs::read_to_string(&toml_path)?;
            if !existing.contains("[source]") {
                fs::write(
                    &toml_path,
                    format!("{}\n{}", existing.trim_end(), source_block),
                )?;
            }
        } else {
            let contents = format!(
                "[module]\n\
                name        = \"{name}\"\n\
                description = \"Installed from {url}\"\n\
                version     = \"1.0.0\"\n\
                deps        = []\n\
                tags        = []\n\
                requires_cmd     = []\n\
                requires_any_cmd = []\n\
                implicit    = false\n\
                \n\
                [api]\n\
                functions    = []\n\
                variables    = []\n\
                defer_on_cmd = false\n\
                \n\
                {source}",
                name = final_name,
                url = parsed.url,
                source = source_block
            );
            fs::write(module_directory.join("module.toml"), contents)?;
        }

        Self::write_init(&module_directory, parsed, &directory_name)?;
        println!("{} installed '{}'\n", "✓".bold().green(), final_name);
        println!(
            "{} Run {} to activate in your current session\n",
            "=>".bold().blue(),
            "gai reload".bold()
        );

        Ok(())
    }

    fn install_collection(
        collection_dirs: &[PathBuf],
        parsed: &ParsedSpec,
        source_block: &str,
        modules: &[DiscoveredModule],
        directory: &Path,
        loader: &Loader,
    ) -> Result<()> {
        println!(
            "  {:<12} {}\n",
            "collection:".dimmed(),
            format!("{} modules detected", collection_dirs.len()).cyan()
        );

        let mut incoming: Vec<(String, &Path)> = Vec::with_capacity(collection_dirs.len());
        for subdir in collection_dirs {
            let manifest_path = subdir.join("module.toml");

            let read_error_context =
                || format!("Failed to read module name from {}", subdir.display());
            let name = Self::toml_name(&manifest_path).with_context(read_error_context)?;

            if !name_valid(&name) {
                bail!(
                    "Module name '{}' in {} is not valid (must match [a-zA-Z_][a-zA-Z0-9_]*).",
                    name,
                    subdir.display()
                );
            }

            let matches_name = |discovered_module: &DiscoveredModule| {
                discovered_module.manifest.module.name == name
            };
            if modules.iter().any(matches_name) {
                bail!(
                    "Module '{}' already exists. Remove it with 'gai rm {}' before reinstalling.",
                    name,
                    name
                );
            }

            let is_duplicate_incoming = |item: &&(String, &Path)| {
                let (incoming_name, _subdirectory) = *item;
                incoming_name == &name
            };

            if let Some((_, other)) = incoming.iter().find(is_duplicate_incoming) {
                bail!(
                    "Duplicate module name '{}' found in both {} and {}.",
                    name,
                    other.display(),
                    subdir.display()
                );
            }

            incoming.push((name, subdir));
        }

        let matches_directory = |captured_directory: &PathBuf| captured_directory == directory;
        let write_index = loader.dirs.iter().position(matches_directory);

        let start_prefix = Self::next_prefix(modules, write_index, directory);

        let name_length = |(incoming_name, _subdirectory): &(String, &Path)| incoming_name.len();
        let column_width = incoming.iter().map(name_length).max().unwrap_or(12).max(12);

        let mut installed: Vec<String> = Vec::with_capacity(incoming.len());

        for (prefix, (name, subdir)) in (start_prefix..).zip(&incoming) {
            let directory_name = format!("{:02}_{}", prefix, name);
            let module_directory = directory.join(&directory_name);

            if module_directory.exists() {
                bail!("Directory already exists: {}", module_directory.display());
            }

            Self::copy_dir(subdir, &module_directory)?;

            let toml_path = module_directory.join("module.toml");

            let read_error_context = || format!("Failed to read {}", toml_path.display());
            let existing = fs::read_to_string(&toml_path).with_context(read_error_context)?;

            if !existing.contains("[source]") {
                let write_error_context = || format!("Failed to write {}", toml_path.display());
                fs::write(
                    &toml_path,
                    format!("{}\n{}", existing.trim_end(), source_block),
                )
                .with_context(write_error_context)?;
            }

            println!(
                "  {:<width$}  {}",
                name.green(),
                directory_name.dimmed(),
                width = column_width
            );

            installed.push(name.clone());
        }

        println!();
        println!(
            "{} installed {} module(s) from '{}'\n",
            "✓".bold().green(),
            installed.len(),
            parsed.repository
        );
        println!(
            "{} Run {} to activate in your current session\n",
            "=>".bold().blue(),
            "gai reload".bold()
        );

        Ok(())
    }

    fn install_missing(
        dirs: &str,
        target: Option<PathBuf>,
        visited: &mut HashSet<String>,
    ) -> Result<()> {
        let loader = Loader::new(dirs)?;
        let updated_modules = loader.get_modules()?;

        let loaded_names: HashSet<&str> = updated_modules.iter().map(Self::module_name).collect();

        for module in &updated_modules {
            for dependency in &*module.manifest.module.deps {
                if let Some(ref dep_source) = dependency.source {
                    let dependency_name = dependency.name.as_str();

                    if !loaded_names.contains(dependency_name) {
                        println!(
                            "{} Installing missing dependency '{}' from '{}'...",
                            "->".bold().blue(),
                            dependency.name.green(),
                            dep_source.dimmed()
                        );
                        install_recursive(
                            dirs,
                            dep_source,
                            Some(dependency.name.clone()),
                            None,
                            target.clone(),
                            visited,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    fn write_init(directory: &Path, parsed: &ParsedSpec, directory_name: &str) -> Result<()> {
        if directory.join("init.zsh").exists() {
            return Ok(());
        }

        let main_file = Self::detect_main(directory, &parsed.repository);
        let init_content = match main_file {
            Some(ref file) => format!(
                "# Auto-generated by gaiety install\n\
                # Source: {url}\n\
                \n\
                source \"${{0:h}}/{file}\"\n",
                url = parsed.url,
            ),
            None => format!(
                "# Auto-generated by gaiety install\n\
                # Source: {url}\n\
                # TODO: source the main plugin file, e.g.:\n\
                # source \"${{0:h}}/plugin.zsh\"\n",
                url = parsed.url
            ),
        };
        fs::write(directory.join("init.zsh"), &init_content)?;

        if init_content.contains("TODO") {
            eprintln!(
                "{} Could not auto-detect the main plugin file.\n  Edit {}/init.zsh before running 'gai reload'.",
                "warn:".bold().yellow(),
                directory_name,
            );
        }

        Ok(())
    }

    fn detect_collection(directory: &Path) -> Vec<PathBuf> {
        let valid_module = |path: &PathBuf| -> bool {
            path.is_dir() && path.join("module.toml").exists() && path.join("init.zsh").exists()
        };
        let unpack_dir_entry = |element: std::io::Result<fs::DirEntry>| element.ok();
        let get_path = |entry: fs::DirEntry| entry.path();

        let mut directories: Vec<PathBuf> = fs::read_dir(directory)
            .into_iter()
            .flatten()
            .filter_map(unpack_dir_entry)
            .map(get_path)
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
        let filter_by_directory = |module: &&DiscoveredModule| match write_index {
            Some(index) => module.dir_index == index,
            None => module.path.parent() == Some(directory),
        };
        let get_prefix_order = |module: &DiscoveredModule| module.prefix_order;

        modules
            .iter()
            .filter(filter_by_directory)
            .filter_map(get_prefix_order)
            .max()
            .unwrap_or(0)
            + 1
    }

    fn copy_dir(source: &Path, destination: &Path) -> Result<()> {
        let create_error = || format!("Failed to create directory: {}", destination.display());
        fs::create_dir_all(destination).with_context(create_error)?;

        let read_error = || format!("Failed to read directory: {}", source.display());
        let elements = fs::read_dir(source).with_context(read_error)?;

        for element in elements {
            let process_error =
                || format!("Failed to process directory entry in: {}", source.display());
            let element = element.with_context(process_error)?;

            let source_path = element.path();
            let file_name = element.file_name();
            let destination_path = destination.join(file_name);

            if source_path.is_dir() {
                Self::copy_dir(&source_path, &destination_path)?;
            } else {
                let copy_error = || {
                    format!(
                        "Failed to copy file from {} to {}",
                        source_path.display(),
                        destination_path.display()
                    )
                };
                fs::copy(&source_path, &destination_path).with_context(copy_error)?;
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

        let missing_name_error = || anyhow!("name field not found");
        document["module"]["name"]
            .as_str()
            .map(String::from)
            .ok_or_else(missing_name_error)
    }

    fn detect_main(directory: &Path, repository: &str) -> Option<String> {
        for suffix in Self::SEARCH_SUFFIXES {
            let candidate = format!("{}{}", repository, suffix);
            if directory.join(&candidate).exists() {
                return Some(candidate);
            }
        }

        let is_plugin_zsh = |filename: &String| filename.ends_with(Self::DOT_PLUGIN_ZSH);
        let mut plugin_files = Self::read_filenames(directory).filter(is_plugin_zsh);

        if let Some(first) = plugin_files.next() {
            if plugin_files.next().is_none() {
                return Some(first);
            }
        }

        let is_zsh_excluding_init =
            |filename: &String| filename.ends_with(Self::DOT_ZSH) && filename != "init.zsh";
        let mut zsh_files = Self::read_filenames(directory).filter(is_zsh_excluding_init);

        if let Some(first) = zsh_files.next() {
            if zsh_files.next().is_none() {
                return Some(first);
            }
        }

        None
    }

    fn read_filenames(directory: &Path) -> impl Iterator<Item = String> {
        let unpack_dir_entry = |element: std::io::Result<fs::DirEntry>| element.ok();
        let is_file_entry = |element: &fs::DirEntry| element.path().is_file();
        let get_filename = |element: fs::DirEntry| element.file_name().into_string().ok();

        fs::read_dir(directory)
            .into_iter()
            .flatten()
            .filter_map(unpack_dir_entry)
            .filter(is_file_entry)
            .filter_map(get_filename)
    }

    fn module_name(module: &DiscoveredModule) -> &str {
        &module.manifest.module.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_to_module() {
        assert_eq!(
            repo_to_module("zsh-syntax-highlighting"),
            "zsh_syntax_highlighting"
        );
        assert_eq!(repo_to_module("MyPlugin.zsh"), "myplugin_zsh");
        assert_eq!(repo_to_module("123plugin"), "_123plugin");
    }

    #[test]
    fn test_parse_spec_github_shorthand() {
        let p = ParsedSpec::new("zsh-users/zsh-syntax-highlighting", None).unwrap();
        assert_eq!(
            p.url,
            "https://github.com/zsh-users/zsh-syntax-highlighting.git"
        );
        assert_eq!(p.repository, "zsh-syntax-highlighting");
        assert!(p.branch.is_none());
    }

    #[test]
    fn test_parse_spec_with_inline_branch() {
        let p = ParsedSpec::new("zsh-users/zsh-autosuggestions@develop", None).unwrap();
        assert_eq!(p.branch.as_deref(), Some("develop"));
    }

    #[test]
    fn test_parse_spec_branch_override_wins() {
        let p =
            ParsedSpec::new("zsh-users/zsh-autosuggestions@develop", Some("main".into())).unwrap();
        assert_eq!(p.branch.as_deref(), Some("main"));
    }

    #[test]
    fn test_parse_spec_gitlab() {
        let p = ParsedSpec::new("gitlab:user/repo", None).unwrap();
        assert!(p.url.starts_with("https://gitlab.com/"));
    }

    #[test]
    fn test_parse_spec_full_url() {
        let p = ParsedSpec::new("https://github.com/user/repo.git", None).unwrap();
        assert_eq!(p.url, "https://github.com/user/repo.git");
        assert_eq!(p.repository, "repo");
    }

    #[test]
    fn test_parse_spec_invalid() {
        assert!(ParsedSpec::new("notaspec", None).is_err());
    }

    #[test]
    fn test_is_valid_name() {
        assert!(name_valid("zsh_syntax_highlighting"));
        assert!(name_valid("_plugin"));
        assert!(!name_valid("123bad"));
        assert!(!name_valid("has-dash"));
        assert!(!name_valid(""));
    }

    #[test]
    fn test_detect_collection() {
        let tmp = std::env::temp_dir().join(format!(
            "gai_test_collect_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&tmp).unwrap();

        for name in &["alpha", "beta"] {
            let d = tmp.join(name);
            fs::create_dir_all(&d).unwrap();
            fs::write(
                d.join("module.toml"),
                format!("[module]\nname=\"{}\"\nversion=\"1.0.0\"", name),
            )
            .unwrap();
            fs::write(d.join("init.zsh"), "").unwrap();
        }

        fs::create_dir_all(tmp.join("not_a_module")).unwrap();

        let extract_filename =
            |path: &PathBuf| path.file_name().unwrap().to_string_lossy().into_owned();

        let found = Helper::detect_collection(&tmp);
        assert_eq!(found.len(), 2);
        let names: Vec<_> = found.iter().map(extract_filename).collect();
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));

        let _ = fs::remove_dir_all(&tmp);
    }
}
