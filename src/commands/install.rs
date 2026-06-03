use crate::core::Loader;
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn run(
    dirs: String,
    spec: String,
    name_override: Option<String>,
    branch_override: Option<String>,
    target: Option<PathBuf>,
) -> Result<()> {
    require_git()?;

    let parsed = parse_spec(&spec, branch_override)?;

    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let write_dir = target.as_ref().unwrap_or_else(|| loader.default_write_dir()).clone();

    let tmp_name = format!(".gai_install_tmp_{}", std::process::id());
    let tmp_dir = write_dir.join(&tmp_name);

    if tmp_dir.exists() {
        fs::remove_dir_all(&tmp_dir)
            .with_context(|| format!("Failed to remove stale temp dir: {}", tmp_dir.display()))?;
    }

    println!("\n{} {}\n", "::".bold().cyan(), "Install Package".bold().cyan());
    println!("  {:<12} {}", "source:".dimmed(), parsed.url.dimmed());
    if let Some(ref b) = parsed.branch {
        println!("  {:<12} {}", "branch:".dimmed(), b.dimmed());
    }

    let mut clone_args: Vec<String> = vec!["clone".into(), "--depth=1".into()];
    if let Some(ref b) = parsed.branch {
        clone_args.push("--branch".into());
        clone_args.push(b.clone());
    }
    clone_args.push(parsed.url.clone());
    clone_args.push(tmp_dir.to_string_lossy().into_owned());

    let status = Command::new("git")
        .args(&clone_args)
        .status()
        .context("Failed to run git")?;

    if !status.success() {
        if tmp_dir.exists() {
            let _ = fs::remove_dir_all(&tmp_dir);
        }
        bail!("git clone failed — see output above");
    }

    let pin = head_commit(&tmp_dir);
    let source_block = build_source_block(&parsed, pin.as_deref());

    let collection_dirs = detect_collection(&tmp_dir);
    let is_single = tmp_dir.join("module.toml").exists();

    let result = if is_single || collection_dirs.is_empty() {
        install_single(
            &tmp_dir,
            &parsed,
            &source_block,
            &modules,
            &write_dir,
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
        install_collection(&tmp_dir, &collection_dirs, &parsed, &source_block, &modules, &write_dir, &loader)
    };

    let _ = fs::remove_dir_all(&tmp_dir);
    result
}

fn install_single(
    tmp_dir: &Path,
    parsed: &ParsedSpec,
    source_block: &str,
    modules: &[crate::core::types::DiscoveredModule],
    write_dir: &Path,
    loader: &crate::core::Loader,
    name_override: Option<String>,
) -> Result<()> {
    let module_name = name_override.unwrap_or_else(|| repo_to_module_name(&parsed.repo_name));

    if !is_valid_name(&module_name) {
        bail!(
            "Derived module name '{}' is not valid (must match [a-zA-Z_][a-zA-Z0-9_]*).\n  Use --name to provide a valid name.",
            module_name
        );
    }

    if modules.iter().any(|m| m.manifest.module.name == module_name) {
        bail!(
            "Module '{}' already exists. Use 'gai update {}' to pull the latest version.",
            module_name, module_name
        );
    }

    let write_dir_index = loader.dirs.iter().position(|d| d == write_dir);
    let max_prefix = next_prefix(modules, write_dir_index, write_dir);
    let dir_name = format!("{:02}_{}", max_prefix, module_name);
    let module_dir = write_dir.join(&dir_name);

    if module_dir.exists() {
        bail!("Directory already exists: {}", module_dir.display());
    }

    println!("  {:<12} {}", "module:".dimmed(), module_name.green());
    println!("  {:<12} {}\n", "dest:".dimmed(), module_dir.display().to_string().dimmed());

    copy_dir(tmp_dir, &module_dir)?;

    let has_toml = module_dir.join("module.toml").exists();
    let has_init = module_dir.join("init.zsh").exists();

    let final_name = if has_toml {
        read_toml_name(&module_dir.join("module.toml")).unwrap_or_else(|_| module_name.clone())
    } else {
        module_name.clone()
    };

    if has_toml {
        let toml_path = module_dir.join("module.toml");
        let existing = fs::read_to_string(&toml_path)?;
        if !existing.contains("[source]") {
            fs::write(&toml_path, format!("{}\n{}", existing.trim_end(), source_block))?;
        }
    } else {
        let content = format!(
            "[module]\n\
             name        = \"{name}\"\n\
             description = \"Installed from {url}\"\n\
             version     = \"1.0.0\"\n\
             deps        = []\n\
             tags        = []\n\
             requires_cmd     = []\n\
             requires_any_cmd = []\n\
             \n\
             [api]\n\
             functions    = []\n\
             variables    = []\n\
             defer_on_cmd = false\n\
             \n\
             {source}",
            name = final_name,
            url = parsed.url,
            source = source_block,
        );
        fs::write(module_dir.join("module.toml"), content)?;
    }

    if !has_init {
        let main_file = detect_main_file(&module_dir, &parsed.repo_name);
        let init_content = match main_file {
            Some(ref f) => format!(
                "# Auto-generated by gaiety install\n\
                 # Source: {url}\n\
                 \n\
                 source \"${{0:h}}/{file}\"\n",
                url = parsed.url,
                file = f,
            ),
            None => format!(
                "# Auto-generated by gaiety install\n\
                 # Source: {url}\n\
                 # TODO: source the main plugin file, e.g.:\n\
                 # source \"${{0:h}}/plugin.zsh\"\n",
                url = parsed.url,
            ),
        };
        fs::write(module_dir.join("init.zsh"), &init_content)?;

        if init_content.contains("TODO") {
            eprintln!(
                "{} Could not auto-detect the main plugin file.\n  Edit {}/init.zsh before running 'gai reload'.",
                "warn:".bold().yellow(),
                dir_name,
            );
        }
    }

    println!("{} installed '{}'\n", "✓".bold().green(), final_name);
    println!(
        "{} Run {} to activate in your current session\n",
        "=>".bold().blue(),
        "gai reload".bold()
    );

    Ok(())
}

fn install_collection(
    tmp_dir: &Path,
    collection_dirs: &[PathBuf],
    parsed: &ParsedSpec,
    source_block: &str,
    modules: &[crate::core::types::DiscoveredModule],
    write_dir: &Path,
    loader: &crate::core::Loader,
) -> Result<()> {
    println!(
        "  {:<12} {}\n",
        "collection:".dimmed(),
        format!("{} modules detected", collection_dirs.len()).cyan()
    );

    let mut incoming: Vec<(String, &Path)> = Vec::new();
    for subdir in collection_dirs {
        let name = read_toml_name(&subdir.join("module.toml"))
            .with_context(|| format!("Failed to read module name from {}", subdir.display()))?;

        if !is_valid_name(&name) {
            bail!(
                "Module name '{}' in {} is not valid (must match [a-zA-Z_][a-zA-Z0-9_]*).",
                name,
                subdir.display()
            );
        }

        if modules.iter().any(|m| m.manifest.module.name == name) {
            bail!(
                "Module '{}' already exists. Remove it with 'gai rm {}' before reinstalling.",
                name, name
            );
        }

        let dupe = incoming.iter().find(|(n, _)| n == &name);
        if let Some((_, other)) = dupe {
            bail!(
                "Duplicate module name '{}' found in both {} and {}.",
                name,
                other.display(),
                subdir.display()
            );
        }

        incoming.push((name, subdir));
    }

    let write_dir_index = loader.dirs.iter().position(|d| d == write_dir);
    let mut prefix = next_prefix(modules, write_dir_index, write_dir);

    let col_w = incoming.iter().map(|(n, _)| n.len()).max().unwrap_or(12).max(12);

    let mut installed: Vec<String> = Vec::new();

    for (name, subdir) in &incoming {
        let dir_name = format!("{:02}_{}", prefix, name);
        let module_dir = write_dir.join(&dir_name);

        if module_dir.exists() {
            bail!("Directory already exists: {}", module_dir.display());
        }

        copy_dir(subdir, &module_dir)?;

        let toml_path = module_dir.join("module.toml");
        let existing = fs::read_to_string(&toml_path)
            .with_context(|| format!("Failed to read {}", toml_path.display()))?;
        if !existing.contains("[source]") {
            fs::write(&toml_path, format!("{}\n{}", existing.trim_end(), source_block))
                .with_context(|| format!("Failed to write {}", toml_path.display()))?;
        }

        println!(
            "  {}  {}",
            format!("{:<width$}", name.green(), width = col_w),
            dir_name.dimmed()
        );

        installed.push(name.clone());
        prefix += 1;
    }

    println!();
    println!(
        "{} installed {} module(s) from '{}'\n",
        "✓".bold().green(),
        installed.len(),
        parsed.repo_name
    );
    println!(
        "{} Run {} to activate in your current session\n",
        "=>".bold().blue(),
        "gai reload".bold()
    );

    Ok(())
}

fn detect_collection(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.join("module.toml").exists()
                && p.join("init.zsh").exists()
        })
        .collect();
    dirs.sort();
    dirs
}

fn next_prefix(
    modules: &[crate::core::types::DiscoveredModule],
    write_dir_index: Option<usize>,
    write_dir: &Path,
) -> u32 {
    modules
        .iter()
        .filter(|m| match write_dir_index {
            Some(idx) => m.dir_index == idx,
            None => m.path.parent().map_or(false, |p| p == write_dir),
        })
        .filter_map(|m| m.prefix_order)
        .max()
        .unwrap_or(0)
        + 1
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir(dst)
        .with_context(|| format!("Failed to create dir: {}", dst.display()))?;
    for entry in fs::read_dir(src)
        .with_context(|| format!("Failed to read dir: {}", src.display()))?
    {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).with_context(|| {
                format!("Failed to copy {} → {}", src_path.display(), dst_path.display())
            })?;
        }
    }
    Ok(())
}

struct ParsedSpec {
    url: String,
    repo_name: String,
    branch: Option<String>,
}

fn parse_spec(spec: &str, branch_override: Option<String>) -> Result<ParsedSpec> {
    let (base, inline_branch) =
        if !spec.starts_with("http://") && !spec.starts_with("https://") {
            if let Some(pos) = spec.rfind('@') {
                (&spec[..pos], Some(spec[pos + 1..].to_string()))
            } else {
                (spec, None)
            }
        } else {
            (spec, None)
        };

    let branch = branch_override.or(inline_branch);

    let (raw_url, repo_name) =
        if base.starts_with("http://") || base.starts_with("https://") {
            let name = base
                .trim_end_matches('/')
                .trim_end_matches(".git")
                .rsplit('/')
                .next()
                .unwrap_or("plugin")
                .to_string();
            (base.to_string(), name)
        } else if let Some(path) = base.strip_prefix("github:") {
            let name = last_segment(path);
            (format!("https://github.com/{}", path), name)
        } else if let Some(path) = base.strip_prefix("gitlab:") {
            let name = last_segment(path);
            (format!("https://gitlab.com/{}", path), name)
        } else if base.contains('/') {
            let name = last_segment(base);
            (format!("https://github.com/{}", base), name)
        } else {
            bail!(
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

    let url = format!("{}.git", raw_url.trim_end_matches(".git"));
    Ok(ParsedSpec { url, repo_name, branch })
}

fn last_segment(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("plugin")
        .to_string()
}

pub fn repo_to_module_name(repo_name: &str) -> String {
    let s: String = repo_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_lowercase();

    if s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{}", s)
    } else {
        s
    }
}

pub fn is_valid_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn head_commit(dir: &Path) -> Option<String> {
    Command::new("git")
        .args(["-C", &dir.to_string_lossy(), "rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn build_source_block(parsed: &ParsedSpec, pin: Option<&str>) -> String {
    let mut s = String::from("[source]\n");
    s.push_str(&format!("url    = \"{}\"\n", parsed.url));
    if let Some(ref b) = parsed.branch {
        s.push_str(&format!("branch = \"{}\"\n", b));
    }
    if let Some(p) = pin {
        s.push_str(&format!("pin    = \"{}\"\n", p));
    }
    s
}

fn read_toml_name(path: &Path) -> Result<String> {
    let content = fs::read_to_string(path)?;
    let doc: toml_edit::DocumentMut = content.parse().context("Failed to parse TOML")?;
    doc["module"]["name"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("name field not found"))
}

fn require_git() -> Result<()> {
    if Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        bail!("git not found in PATH — required for gai install");
    }
    Ok(())
}

fn detect_main_file(dir: &Path, repo_name: &str) -> Option<String> {
    for suffix in &[".zsh", ".plugin.zsh", ".zsh-theme"] {
        let candidate = format!("{}{}", repo_name, suffix);
        if dir.join(&candidate).exists() {
            return Some(candidate);
        }
    }

    let plugin_files: Vec<String> = read_filenames(dir)
        .filter(|n| n.ends_with(".plugin.zsh"))
        .collect();
    if plugin_files.len() == 1 {
        return Some(plugin_files.into_iter().next().unwrap());
    }

    let zsh_files: Vec<String> = read_filenames(dir)
        .filter(|n| n.ends_with(".zsh") && n != "init.zsh")
        .collect();
    if zsh_files.len() == 1 {
        return Some(zsh_files.into_iter().next().unwrap());
    }

    None
}

fn read_filenames(dir: &Path) -> impl Iterator<Item = String> {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_to_module_name() {
        assert_eq!(repo_to_module_name("zsh-syntax-highlighting"), "zsh_syntax_highlighting");
        assert_eq!(repo_to_module_name("MyPlugin.zsh"), "myplugin_zsh");
        assert_eq!(repo_to_module_name("123plugin"), "_123plugin");
    }

    #[test]
    fn test_parse_spec_github_shorthand() {
        let p = parse_spec("zsh-users/zsh-syntax-highlighting", None).unwrap();
        assert_eq!(p.url, "https://github.com/zsh-users/zsh-syntax-highlighting.git");
        assert_eq!(p.repo_name, "zsh-syntax-highlighting");
        assert!(p.branch.is_none());
    }

    #[test]
    fn test_parse_spec_with_inline_branch() {
        let p = parse_spec("zsh-users/zsh-autosuggestions@develop", None).unwrap();
        assert_eq!(p.branch.as_deref(), Some("develop"));
    }

    #[test]
    fn test_parse_spec_branch_override_wins() {
        let p = parse_spec("zsh-users/zsh-autosuggestions@develop", Some("main".into())).unwrap();
        assert_eq!(p.branch.as_deref(), Some("main"));
    }

    #[test]
    fn test_parse_spec_gitlab() {
        let p = parse_spec("gitlab:user/repo", None).unwrap();
        assert!(p.url.starts_with("https://gitlab.com/"));
    }

    #[test]
    fn test_parse_spec_full_url() {
        let p = parse_spec("https://github.com/user/repo.git", None).unwrap();
        assert_eq!(p.url, "https://github.com/user/repo.git");
        assert_eq!(p.repo_name, "repo");
    }

    #[test]
    fn test_parse_spec_invalid() {
        assert!(parse_spec("notaspec", None).is_err());
    }

    #[test]
    fn test_is_valid_name() {
        assert!(is_valid_name("zsh_syntax_highlighting"));
        assert!(is_valid_name("_plugin"));
        assert!(!is_valid_name("123bad"));
        assert!(!is_valid_name("has-dash"));
        assert!(!is_valid_name(""));
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
            fs::write(d.join("module.toml"), format!("[module]\nname=\"{}\"\nversion=\"1.0.0\"", name)).unwrap();
            fs::write(d.join("init.zsh"), "").unwrap();
        }

        fs::create_dir_all(tmp.join("not_a_module")).unwrap();

        let found = detect_collection(&tmp);
        assert_eq!(found.len(), 2);
        let names: Vec<_> = found.iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));

        let _ = fs::remove_dir_all(&tmp);
    }
}
