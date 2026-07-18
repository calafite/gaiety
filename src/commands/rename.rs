use crate::core::file_guard::*;
use crate::core::loader::Loader;
use crate::core::manifest::Dependency;
use crate::core::types::DiscoveredModule;
use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::DocumentMut;

pub fn run(directories: String, old_name: String, new_name: String) -> Result<()> {
    let loader_context = || format!("Failed to initialize loader for: {}", directories);
    let loader = Loader::new(&directories).with_context(loader_context)?;

    let modules_context = || "Failed to retrieve modules".to_string();
    let modules = loader.get_modules().with_context(modules_context)?;

    let find_predicate =
        |discovered_module: &&DiscoveredModule| discovered_module.manifest.module.name == old_name;
    let target_module = modules
        .iter()
        .find(find_predicate)
        .ok_or_else(|| anyhow::anyhow!("Module '{}' not found.", old_name))?;

    let matches_new_name =
        |discovered_module: &DiscoveredModule| discovered_module.manifest.module.name == new_name;
    if modules.iter().any(matches_new_name) {
        bail!("Module '{}' already exists.", new_name);
    }

    let old_directory = &target_module.path;
    let directory_name = old_directory.file_name().unwrap().to_string_lossy();
    let new_directory_name = match directory_name.splitn(2, '_').collect::<Vec<_>>().as_slice() {
        [prefix, _] => format!("{}_{}", prefix, new_name),
        _ => new_name.clone(),
    };
    let parent_directory = old_directory.parent().unwrap();
    let new_directory = parent_directory.join(&new_directory_name);

    let own_toml_path = old_directory.join("module.toml");
    let read_toml_context = || format!("Failed to read {}", own_toml_path.display());
    let own_content = fs::read_to_string(&own_toml_path).with_context(read_toml_context)?;

    let rewrite_toml_context = || format!("Failed to rewrite {}", own_toml_path.display());
    let own_updated =
        Helper::set_module_name(&own_content, &new_name).with_context(rewrite_toml_context)?;

    let dependent_rewrites = Helper::collect_dependent_rewrites(&modules, &old_name, &new_name)?;

    if new_directory.exists() {
        bail!(
            "Destination already exists: {}. Remove it and retry.",
            new_directory.display()
        );
    }

    println!(
        "\n{} {}\n",
        "::".bold().cyan(),
        "Rename Module".bold().cyan()
    );
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
        directory_name.to_string().dimmed(),
        "→".dimmed(),
        new_directory_name.dimmed()
    );
    if !dependent_rewrites.is_empty() {
        fn extract_name(entry: &(String, PathBuf, String)) -> &str {
            entry.0.as_str()
        }
        let dependency_names = dependent_rewrites
            .iter()
            .map(extract_name)
            .collect::<Vec<_>>()
            .join(", ");
        println!("  {:<10} {}", "deps:".dimmed(), dependency_names.dimmed());
    }
    println!();

    let mut dependency_temporaries: Vec<(PathBuf, PathBuf)> =
        Vec::with_capacity(dependent_rewrites.len());
    let mut dependency_temp_guard = TempFilesGuard::new();

    for (_, target_path, updated) in &dependent_rewrites {
        let temporary_file_name = target_path.with_file_name(".module.toml.gai_tmp");

        let write_error_context = || {
            format!(
                "Failed to write dep temp at {}",
                temporary_file_name.display()
            )
        };
        fs::write(&temporary_file_name, updated).with_context(write_error_context)?;

        dependency_temp_guard.add(temporary_file_name.clone());
        dependency_temporaries.push((temporary_file_name, target_path.clone()));
    }

    let temporary_dir_name =
        crate::core::common::temporary_name(&format!("rename_{}", new_directory_name));
    let temporary_directory = parent_directory.join(temporary_dir_name);

    if temporary_directory.exists() {
        let remove_context = || {
            format!(
                "Failed to remove stale temp dir: {}",
                temporary_directory.display()
            )
        };
        fs::remove_dir_all(&temporary_directory).with_context(remove_context)?;
    }

    let mut temporary_guard = TempDirGuard::new(temporary_directory.clone());

    let copy_context = || {
        format!(
            "Failed to copy to temp dir: {}",
            temporary_directory.display()
        )
    };
    Helper::copy_dir(old_directory, &temporary_directory).with_context(copy_context)?;

    let write_toml_context = || {
        format!(
            "Failed to write module.toml into {}",
            temporary_directory.display()
        )
    };
    fs::write(temporary_directory.join("module.toml"), &own_updated)
        .with_context(write_toml_context)?;

    let rename_context = || {
        format!(
            "Failed to rename {} → {}",
            temporary_directory.display(),
            new_directory.display()
        )
    };
    fs::rename(&temporary_directory, &new_directory).with_context(rename_context)?;
    temporary_guard.defuse();

    for (temporary_path, target_path) in &dependency_temporaries {
        let commit_context = || format!("Failed to commit dep TOML at {}", target_path.display());
        fs::rename(temporary_path, target_path).with_context(commit_context)?;
    }
    dependency_temp_guard.defuse();

    let remove_original_context =
        || format!("Failed to remove original dir: {}", old_directory.display());
    fs::remove_dir_all(old_directory).with_context(remove_original_context)?;

    println!(
        "{} renamed '{}' → '{}'\n",
        "✓".bold().green(),
        old_name,
        new_name
    );
    Ok(())
}

struct Helper;

impl Helper {
    fn collect_dependent_rewrites(
        modules: &[DiscoveredModule],
        old_name: &str,
        new_name: &str,
    ) -> Result<Vec<(String, PathBuf, String)>> {
        let mut dependent_rewrites = Vec::new();
        for dependency_module in modules {
            if dependency_module.manifest.module.name == old_name {
                continue;
            }

            let matches_old_name = |dependency: &Dependency| dependency.name == old_name;
            let has_dependency = dependency_module
                .manifest
                .module
                .deps
                .iter()
                .any(matches_old_name);

            if has_dependency {
                let path = dependency_module.path.join("module.toml");

                let read_context = || format!("Failed to read {}", path.display());
                let content = fs::read_to_string(&path).with_context(read_context)?;

                let rewrite_context = || format!("Failed to rewrite dep in {}", path.display());
                let updated =
                    Self::rename_dep(&content, old_name, new_name).with_context(rewrite_context)?;

                dependent_rewrites.push((
                    dependency_module.manifest.module.name.clone(),
                    path,
                    updated,
                ));
            }
        }
        Ok(dependent_rewrites)
    }

    fn copy_dir(source: &Path, destination: &Path) -> Result<()> {
        let create_context = || format!("Failed to create dir: {}", destination.display());
        fs::create_dir(destination).with_context(create_context)?;

        let read_context = || format!("Failed to read dir: {}", source.display());
        let entries = fs::read_dir(source).with_context(read_context)?;

        for entry in entries {
            let entry = entry?;
            let source_path = entry.path();
            let destination_path = destination.join(entry.file_name());

            if source_path.is_dir() {
                Self::copy_dir(&source_path, &destination_path)?;
            } else {
                let copy_context = || {
                    format!(
                        "Failed to copy {} → {}",
                        source_path.display(),
                        destination_path.display()
                    )
                };
                fs::copy(&source_path, &destination_path).with_context(copy_context)?;
            }
        }
        Ok(())
    }

    fn set_module_name(content: &str, new_name: &str) -> Result<String> {
        let mut document = content
            .parse::<DocumentMut>()
            .context("Failed to parse TOML")?;
        document["module"]["name"] = toml_edit::value(new_name);
        Ok(document.to_string())
    }

    fn rename_dep(content: &str, old_name: &str, new_name: &str) -> Result<String> {
        let mut document = content
            .parse::<DocumentMut>()
            .context("Failed to parse TOML")?;

        if let Some(dependencies) = document
            .get_mut("module")
            .and_then(|module| module.get_mut("deps"))
        {
            if let Some(tables) = dependencies.as_array_of_tables_mut() {
                for dependency in tables.iter_mut() {
                    let matches_name =
                        dependency.get("name").and_then(|value| value.as_str()) == Some(old_name);
                    if matches_name {
                        dependency["name"] = toml_edit::value(new_name);
                    }
                }
            } else if let Some(array) = dependencies.as_array_mut() {
                for element in array.iter_mut() {
                    if let Some(table) = element.as_inline_table_mut() {
                        let matches_name =
                            table.get("name").and_then(|value| value.as_str()) == Some(old_name);
                        if matches_name {
                            table.insert("name", toml_edit::Value::from(new_name));
                        }
                    }
                }
            }
        }

        Ok(document.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_module_name() {
        let toml_content = "[module]\nname = \"old\"\nversion = \"1.0.0\"";
        let updated = Helper::set_module_name(toml_content, "new").unwrap();
        assert!(updated.contains("name = \"new\""));
    }

    #[test]
    fn test_rename_dep() {
        let toml_content = r#"
            [module]
            name = "my_module"
            deps = [
                { name = "old_dep", version = "1.0" }
            ]
            "#;
        let updated = Helper::rename_dep(toml_content, "old_dep", "new_dep").unwrap();
        assert!(updated.contains("new_dep"));
        assert!(!updated.contains("old_dep"));
    }
}
