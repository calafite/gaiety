use crate::loader::Loader;
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::DocumentMut;

pub fn run(dirs: String, old_name: String, new_name: String) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let m = modules
        .iter()
        .find(|m| m.manifest.module.name == old_name)
        .ok_or_else(|| anyhow::anyhow!("Module '{}' not found.", old_name))?;

    if modules.iter().any(|m| m.manifest.module.name == new_name) {
        bail!("Module '{}' already exists.", new_name);
    }

    let old_dir = &m.path;
    let dir_name = old_dir.file_name().unwrap().to_string_lossy();
    let new_dir_name = match dir_name.splitn(2, '_').collect::<Vec<_>>().as_slice() {
        [prefix, _] => format!("{}_{}", prefix, new_name),
        _ => new_name.clone(),
    };
    let parent = old_dir.parent().unwrap();
    let new_dir = parent.join(&new_dir_name);

    let own_toml_path = old_dir.join("module.toml");
    let own_content = fs::read_to_string(&own_toml_path)
        .with_context(|| format!("Failed to read {}", own_toml_path.display()))?;
    let own_updated = set_module_name(&own_content, &new_name)
        .with_context(|| format!("Failed to rewrite {}", own_toml_path.display()))?;

    let mut dependent_rewrites: Vec<(String, PathBuf, String)> = Vec::new();
    for dep_module in &modules {
        if dep_module.manifest.module.name == old_name {
            continue;
        }
        if dep_module.manifest.module.deps.iter().any(|d| d.name == old_name) {
            let path = dep_module.path.join("module.toml");
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            let updated = rename_dep(&content, &old_name, &new_name)
                .with_context(|| format!("Failed to rewrite dep in {}", path.display()))?;
            dependent_rewrites.push((dep_module.manifest.module.name.clone(), path, updated));
        }
    }

    if new_dir.exists() {
        bail!(
            "Destination already exists: {}. Remove it and retry.",
            new_dir.display()
        );
    }

    println!("\n{} {}\n", "::".bold().cyan(), "Rename Module".bold().cyan());
    println!(
        "  {:<10} {} {} {}",
        "name:".dimmed(),
        old_name.green(),
        "→".dimmed(),
        new_name.green()
    );
    println!(
        "  {:<10} {} {} {}",
        "dir:".dimmed(),
        dir_name.to_string().dimmed(),
        "→".dimmed(),
        new_dir_name.dimmed()
    );
    if !dependent_rewrites.is_empty() {
        let dep_names = dependent_rewrites
            .iter()
            .map(|(name, _, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        println!("  {:<10} {}", "deps:".dimmed(), dep_names.dimmed());
    }
    println!();

    let mut dep_temps: Vec<(PathBuf, PathBuf)> = Vec::new(); // (tmp, target)
    let mut dep_temp_guard = TempFilesGuard::new();

    for (_, target_path, updated) in &dependent_rewrites {
        let tmp = target_path.with_file_name(".module.toml.gai_tmp");
        fs::write(&tmp, updated)
            .with_context(|| format!("Failed to write dep temp at {}", tmp.display()))?;
        dep_temp_guard.add(tmp.clone());
        dep_temps.push((tmp, target_path.clone()));
    }

    let tmp_dir = parent.join(format!(".gai_rename_tmp_{}", new_dir_name));
    if tmp_dir.exists() {
        fs::remove_dir_all(&tmp_dir)
            .with_context(|| format!("Failed to remove stale temp dir: {}", tmp_dir.display()))?;
    }

    let mut tmp_guard = TempDirGuard::new(tmp_dir.clone());

    copy_dir(old_dir, &tmp_dir)
        .with_context(|| format!("Failed to copy to temp dir: {}", tmp_dir.display()))?;

    fs::write(tmp_dir.join("module.toml"), &own_updated)
        .with_context(|| format!("Failed to write module.toml into {}", tmp_dir.display()))?;

    fs::rename(&tmp_dir, &new_dir).with_context(|| {
        format!(
            "Failed to rename {} → {}",
            tmp_dir.display(),
            new_dir.display()
        )
    })?;
    tmp_guard.defuse();
 
    for (tmp_path, target_path) in &dep_temps {
        fs::rename(tmp_path, target_path).with_context(|| {
            format!("Failed to commit dep TOML at {}", target_path.display())
        })?;
    }
    dep_temp_guard.defuse();

    fs::remove_dir_all(old_dir)
        .with_context(|| format!("Failed to remove original dir: {}", old_dir.display()))?;

    println!("{} renamed '{}' → '{}'\n", "✓".bold().green(), old_name, new_name);
    Ok(())
}

//guards
//
struct TempDirGuard {
    path: PathBuf,
    active: bool,
}

impl TempDirGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, active: true }
    }

    fn defuse(&mut self) {
        self.active = false;
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if self.active && self.path.exists() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

struct TempFilesGuard {
    paths: Vec<PathBuf>,
    active: bool,
}

impl TempFilesGuard {
    fn new() -> Self {
        Self {
            paths: Vec::new(),
            active: true,
        }
    }

    fn add(&mut self, path: PathBuf) {
        self.paths.push(path);
    }

    fn defuse(&mut self) {
        self.active = false;
    }
}

impl Drop for TempFilesGuard {
    fn drop(&mut self) {
        if self.active {
            for path in &self.paths {
                let _ = fs::remove_file(path);
            }
        }
    }
}

//helpers

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
                format!(
                    "Failed to copy {} → {}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn set_module_name(content: &str, new_name: &str) -> Result<String> {
    let mut doc = content
        .parse::<DocumentMut>()
        .context("Failed to parse TOML")?;
    doc["module"]["name"] = toml_edit::value(new_name);
    Ok(doc.to_string())
}

fn rename_dep(content: &str, old_name: &str, new_name: &str) -> Result<String> {
    let mut doc = content
        .parse::<DocumentMut>()
        .context("Failed to parse TOML")?;

    if let Some(deps) = doc
        .get_mut("module")
        .and_then(|m| m.get_mut("deps"))
        .and_then(|d| d.as_array_of_tables_mut())
    {
        for dep in deps.iter_mut() {
            if dep.get("name").and_then(|v| v.as_str()) == Some(old_name) {
                dep["name"] = toml_edit::value(new_name);
            }
        }
    } else if let Some(deps) = doc
        .get_mut("module")
        .and_then(|m| m.get_mut("deps"))
        .and_then(|d| d.as_array_mut())
    {
        for dep in deps.iter_mut() {
            if let Some(tbl) = dep.as_inline_table_mut() {
                if tbl.get("name").and_then(|v| v.as_str()) == Some(old_name) {
                    tbl.insert("name", toml_edit::Value::from(new_name));
                }
            }
        }
    }

    Ok(doc.to_string())
}
