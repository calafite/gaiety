use crate::loader::Loader;
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use toml_edit::DocumentMut;

pub fn run(dirs: String, old_name: String, new_name: String) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    let target = modules.iter().find(|m| m.manifest.module.name == old_name);

    let m = match target {
        Some(m) => m,
        None => bail!("Module '{}' not found.", old_name),
    };

    if modules.iter().any(|m| m.manifest.module.name == new_name) {
        bail!("Module '{}' already exists.", new_name);
    }

    let old_dir = &m.path;
    let dir_name = old_dir.file_name().unwrap().to_string_lossy();
    let new_dir_name = match dir_name.splitn(2, '_').collect::<Vec<_>>().as_slice() {
        [prefix, _] => format!("{}_{}", prefix, new_name),
        _ => new_name.clone(),
    };
    let new_dir = old_dir.parent().unwrap().join(&new_dir_name);

    let own_toml_path = old_dir.join("module.toml");
    let own_content = fs::read_to_string(&own_toml_path)
        .with_context(|| format!("Failed to read {}", own_toml_path.display()))?;
    let own_updated = set_module_name(&own_content, &new_name)
        .with_context(|| format!("Failed to rewrite {}", own_toml_path.display()))?;
 
    let mut dependent_rewrites: Vec<(std::path::PathBuf, String, String)> = Vec::new();
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
            dependent_rewrites.push((path, content, updated));
        }
    }

    if new_dir.exists() {
        bail!(
            "Destination already exists: {}. Remove it and retry.",
            new_dir.display()
        );
    }

    copy_dir(old_dir, &new_dir)
        .with_context(|| format!("Failed to copy {} → {}", old_dir.display(), new_dir.display()))?;
    println!("{} copied dir: {} → {}", "↻".bold().blue(), dir_name, new_dir_name);

    let write_result = (|| -> Result<()> {
        let new_toml_path = new_dir.join("module.toml");
        fs::write(&new_toml_path, own_updated)
            .with_context(|| format!("Failed to write {}", new_toml_path.display()))?;
        println!("{} updated: {}", "↻".bold().blue(), new_toml_path.display());

        for (path, _original, updated) in dependent_rewrites {
            fs::write(&path, &updated)
                .with_context(|| format!("Failed to write {}", path.display()))?;
            println!("{} updated dep in: {}", "↻".bold().blue(), path.display());
        }
        Ok(())
    })();

    if let Err(e) = write_result {
        let _ = fs::remove_dir_all(&new_dir);
        return Err(e.context("Rename aborted; original module is unchanged"));
    }

    fs::remove_dir_all(old_dir)
        .with_context(|| format!("Failed to remove original dir: {}", old_dir.display()))?;
    println!("{} removed original: {}", "↻".bold().blue(), dir_name);

    println!("\n{} renamed '{}' → '{}'\n", "✓".bold().green(), old_name, new_name);
    Ok(())
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
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
        // Inline array form: deps = [{ name = "foo", version = ">=1.0.0" }]
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
