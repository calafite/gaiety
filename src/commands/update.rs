use crate::commands::install::head_commit;
use crate::core::Loader;
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use toml_edit::DocumentMut;

pub fn run(dirs: String, module_name: Option<String>) -> Result<()> {
    require_git()?;

    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let managed: Vec<_> = modules
        .iter()
        .filter(|m| m.manifest.source.is_some())
        .filter(|m| match &module_name {
            Some(n) => &m.manifest.module.name == n,
            None => true,
        })
        .collect();

    if managed.is_empty() {
        if let Some(ref n) = module_name {
            bail!(
                "Module '{}' not found or has no [source] section.\n  \
                 Only modules installed via 'gai install' can be updated this way.",
                n
            );
        } else {
            println!(
                "No managed packages found.\n  \
                 Install packages with 'gai install <user/repo>' to track them here."
            );
            return Ok(());
        }
    }

    println!("\n{} {}\n", "::".bold().cyan(), "Update Packages".bold().cyan());

    let col_w = managed
        .iter()
        .map(|m| m.manifest.module.name.len())
        .max()
        .unwrap_or(12)
        .max(12);

    let mut updated = 0usize;
    let mut already_current = 0usize;
    let mut failed = 0usize;

    let (git_modules, collection_modules): (Vec<&&crate::core::types::DiscoveredModule>, Vec<_>) =
        managed.iter().partition(|m| m.path.join(".git").exists());

    for m in &git_modules {
        let name = &m.manifest.module.name;
        let src = m.manifest.source.as_ref().unwrap();

        print!("  {}  ", format!("{:<width$}", name.green(), width = col_w));

        let path_str = m.path.to_string_lossy().into_owned();
        let mut args = vec!["-C".to_string(), path_str, "pull".to_string()];
        if let Some(ref b) = src.branch {
            args.push("origin".to_string());
            args.push(b.clone());
        }

        let out = Command::new("git")
            .args(&args)
            .env("LC_ALL", "C")
            .output();

        match out {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("Already up to date") || stdout.contains("Already up-to-date") {
                    println!("{}", "up to date".dimmed());
                    already_current += 1;
                } else {
                    if let Some(ref p) = head_commit(&m.path) {
                        let _ = update_pin_in_toml(&m.path.join("module.toml"), p);
                    }
                    println!("{}", "updated".bold().green());
                    updated += 1;
                }
            }
            Ok(output) => {
                println!("{}", "failed".bold().red());
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("    {}", stderr.trim());
                failed += 1;
            }
            Err(e) => {
                println!("{}", "failed".bold().red());
                eprintln!("    {}", e);
                failed += 1;
            }
        }
    }

    let mut by_url: HashMap<String, Vec<&&crate::core::types::DiscoveredModule>> = HashMap::new();
    for m in &collection_modules {
        let url = m.manifest.source.as_ref().unwrap().url.clone();
        by_url.entry(url).or_default().push(m);
    }

    for (url, group) in &by_url {
        let branch = group[0].manifest.source.as_ref().unwrap().branch.clone();
        let old_pin = group[0].manifest.source.as_ref().unwrap().pin.clone();

        let parent = group[0].path.parent().unwrap();
        let tmp_dir = parent.join(format!(".gai_update_tmp_{}", std::process::id()));

        if tmp_dir.exists() {
            fs::remove_dir_all(&tmp_dir)
                .with_context(|| format!("Failed to remove stale temp dir: {}", tmp_dir.display()))?;
        }

        let mut args = vec!["clone".to_string()];
        if let Some(ref b) = branch {
            args.push("-b".to_string());
            args.push(b.clone());
        }
        args.push(url.clone());
        args.push(tmp_dir.to_string_lossy().into_owned());

        let clone_status = Command::new("git")
            .args(&args)
            .env("LC_ALL", "C")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output();

        let (clone_ok, new_pin) = match clone_status {
            Ok(output) if output.status.success() => {
                let pin = head_commit(&tmp_dir);
                (true, pin)
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                for m in group {
                    let name = &m.manifest.module.name;
                    println!(
                        "  {}  {}",
                        format!("{:<width$}", name.green(), width = col_w),
                        "clone failed".bold().red()
                    );
                    eprintln!("    {}", stderr.trim());
                    failed += 1;
                }
                let _ = fs::remove_dir_all(&tmp_dir);
                continue;
            }
            Err(e) => {
                for m in group {
                    let name = &m.manifest.module.name;
                    println!(
                        "  {}  {}",
                        format!("{:<width$}", name.green(), width = col_w),
                        "clone failed".bold().red()
                    );
                    eprintln!("    {}", e);
                    failed += 1;
                }
                let _ = fs::remove_dir_all(&tmp_dir);
                continue;
            }
        };

        let already_up = old_pin.is_some() && new_pin.as_deref() == old_pin.as_deref();

        for m in group {
            let name = &m.manifest.module.name;
            print!("  {}  ", format!("{:<width$}", name.green(), width = col_w));

            if !clone_ok {
                continue;
            }

            if already_up {
                println!("{}", "up to date".dimmed());
                already_current += 1;
                continue;
            }

            let subdir = find_collection_subdir(&tmp_dir, name);

            match subdir {
                None => {
                    println!("{}", "not found in repo".bold().red());
                    eprintln!(
                        "    {} module '{}' has no matching subdirectory in {}",
                        "warn:".yellow(),
                        name,
                        url
                    );
                    failed += 1;
                }
                Some(src_path) => {
                    match sync_module_dir(&src_path, &m.path, new_pin.as_deref(), m) {
                        Ok(()) => {
                            println!("{}", "updated".bold().green());
                            updated += 1;
                        }
                        Err(e) => {
                            println!("{}", "failed".bold().red());
                            eprintln!("    {}", e);
                            failed += 1;
                        }
                    }
                }
            }
        }

        let _ = fs::remove_dir_all(&tmp_dir);
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

fn find_collection_subdir(clone_root: &Path, target_name: &str) -> Option<PathBuf> {
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
        if let Ok(content) = fs::read_to_string(&toml_path) {
            if let Ok(doc) = content.parse::<DocumentMut>() {
                if doc["module"]["name"].as_str() == Some(target_name) {
                    return Some(path);
                }
            }
        }
    }
    None
}

fn sync_module_dir(
    src: &Path,
    dest: &Path,
    new_pin: Option<&str>,
    m: &crate::core::types::DiscoveredModule,
) -> Result<()> {
    for entry in fs::read_dir(src)
        .with_context(|| format!("Failed to read {}", src.display()))?
        .flatten()
    {
        let src_path = entry.path();
        let dst_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            sync_module_dir(&src_path, &dst_path, None, m)?;
        } else {
            fs::copy(&src_path, &dst_path).with_context(|| {
                format!("Failed to copy {} → {}", src_path.display(), dst_path.display())
            })?;
        }
    }

    let toml_path = dest.join("module.toml");
    let content = fs::read_to_string(&toml_path)
        .with_context(|| format!("Failed to read {}", toml_path.display()))?;

    let src_entry = m.manifest.source.as_ref().unwrap();
    let updated = rewrite_source_block(&content, &src_entry.url, src_entry.branch.as_deref(), new_pin)
        .with_context(|| format!("Failed to rewrite [source] in {}", toml_path.display()))?;

    fs::write(&toml_path, updated)
        .with_context(|| format!("Failed to write {}", toml_path.display()))?;

    Ok(())
}

fn rewrite_source_block(
    content: &str,
    url: &str,
    branch: Option<&str>,
    pin: Option<&str>,
) -> Result<String> {
    let mut doc: DocumentMut = content
        .parse()
        .context("Failed to parse module.toml as TOML")?;

    let mut tbl = toml_edit::Table::new();
    tbl["url"] = toml_edit::value(url);
    if let Some(b) = branch {
        tbl["branch"] = toml_edit::value(b);
    }
    if let Some(p) = pin {
        tbl["pin"] = toml_edit::value(p);
    }

    doc["source"] = toml_edit::Item::Table(tbl);
    Ok(doc.to_string())
}

fn require_git() -> Result<()> {
    if Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        bail!("git not found in PATH — required for gai update");
    }
    Ok(())
}

fn update_pin_in_toml(path: &Path, new_pin: &str) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    if let Some(source) = doc.get_mut("source") {
        if let Some(tbl) = source.as_table_mut() {
            tbl["pin"] = toml_edit::value(new_pin);
        }
    }
    fs::write(path, doc.to_string())
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn write_temp_toml(content: &str) -> (tempfile_helper::TempDir, PathBuf) {
        let dir = tempfile_helper::tempdir();
        let path = dir.path.join("module.toml");
        fs::write(&path, content).unwrap();
        (dir, path)
    }

    mod tempfile_helper {
        use std::fs;
        use std::path::PathBuf;

        pub struct TempDir {
            pub path: PathBuf,
        }
        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.path);
            }
        }
        pub fn tempdir() -> TempDir {
            let mut p = std::env::temp_dir();
            p.push(format!(
                "gai_test_update_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_micros()
            ));
            fs::create_dir_all(&p).unwrap();
            TempDir { path: p }
        }
    }

    #[test]
    fn test_update_pin_in_toml() {
        let toml = "[module]\nname = \"foo\"\nversion = \"1.0.0\"\n\n[source]\nurl = \"https://example.com/foo.git\"\npin = \"aabbcc\"\n";
        let (_dir, path) = write_temp_toml(toml);
        update_pin_in_toml(&path, "deadbeef").unwrap();
        let updated = fs::read_to_string(&path).unwrap();
        assert!(updated.contains("deadbeef"));
        assert!(!updated.contains("aabbcc"));
    }

    #[test]
    fn test_rewrite_source_block_update_pin() {
        let toml = "[module]\nname = \"bar\"\nversion = \"1.0.0\"\n\n[source]\nurl = \"https://github.com/x/y.git\"\npin = \"oldpin\"\n";
        let result = rewrite_source_block(toml, "https://github.com/x/y.git", None, Some("newpin")).unwrap();
        assert!(result.contains("newpin"));
        assert!(!result.contains("oldpin"));
        assert!(result.contains("https://github.com/x/y.git"));
    }

    #[test]
    fn test_rewrite_source_block_with_branch() {
        let toml = "[module]\nname = \"baz\"\nversion = \"1.0.0\"\n";
        let result = rewrite_source_block(toml, "https://github.com/x/z.git", Some("main"), Some("abc123")).unwrap();
        assert!(result.contains("branch"));
        assert!(result.contains("main"));
        assert!(result.contains("abc123"));
    }

    #[test]
    fn test_find_collection_subdir() {
        let tmp = std::env::temp_dir().join(format!(
            "gai_test_findcol_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&tmp).unwrap();

        let m = tmp.join("01_mymod");
        fs::create_dir_all(&m).unwrap();
        fs::write(
            m.join("module.toml"),
            "[module]\nname = \"mymod\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();

        let found = find_collection_subdir(&tmp, "mymod");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), m);

        let not_found = find_collection_subdir(&tmp, "other");
        assert!(not_found.is_none());

        let _ = fs::remove_dir_all(&tmp);
    }
}
