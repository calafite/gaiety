use crate::commands::install::head_commit;
use crate::core::Loader;
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
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

    for m in &managed {
        let name = &m.manifest.module.name;
        let src = m.manifest.source.as_ref().unwrap();

        print!("  {}  ", format!("{:<width$}", name.green(), width = col_w));

        if !m.path.join(".git").exists() {
            println!("{}", "no .git dir — skipped".yellow());
            continue;
        }

        let path_str = m.path.to_string_lossy().into_owned();
        let mut args = vec!["-C".to_string(), path_str, "pull".to_string()];

        if let Some(ref b) = src.branch {
            args.push("origin".to_string());
            args.push(b.clone());
        }

        let output = Command::new("git").args(&args).output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let already_up = stdout.contains("Already up to date")
                    || stdout.contains("Already up-to-date");

                if already_up {
                    println!("{}", "up to date".dimmed());
                    already_current += 1;
                } else {
                    let new_pin = head_commit(&m.path);
                    if let Some(ref p) = new_pin {
                        let _ = update_pin_in_toml(&m.path.join("module.toml"), p);
                    }
                    println!("{}", "updated".bold().green());
                    updated += 1;
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                println!("{}", "failed".bold().red());
                eprintln!(
                    "    {} {}",
                    "git:".dimmed(),
                    stderr.trim().to_string().dimmed()
                );
                failed += 1;
            }
            Err(e) => {
                println!("{}", "error".bold().red());
                eprintln!("    {}", e);
                failed += 1;
            }
        }
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

fn update_pin_in_toml(path: &std::path::Path, new_pin: &str) -> Result<()> {
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
        use std::path::PathBuf;
        use std::fs;

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
            p.push(format!("gai_test_update_{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_micros()));
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
}
