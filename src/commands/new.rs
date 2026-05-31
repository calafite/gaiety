use crate::loader::Loader;
use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

pub fn run(dir: PathBuf, module_name: String) -> Result<()> {
    if !is_valid_name(&module_name) {
        bail!("Invalid module name: '{}' (must match [a-zA-Z_][a-zA-Z0-9_]*)", module_name);
    }

    let loader = Loader::new(dir.clone());
    let modules = loader.get_modules()?;
    
    if modules.iter().any(|m| m.manifest.module.name == module_name) {
        bail!("Module '{}' is already registered.", module_name);
    }

    let max_prefix = modules
        .iter()
        .filter_map(|m| m.prefix_order)
        .max()
        .unwrap_or(0);
    
    let next_prefix = format!("{:02}", max_prefix + 1);
    let target_dir_name = format!("{}_{}", next_prefix, module_name);
    let target_dir = dir.join(&target_dir_name);

    fs::create_dir_all(&target_dir)?;

    let toml_content = format!(
r#"[module]
name = "{}"
description = "Write a short description here"
version = "1.0.0"
deps = ["core"]
tags = []
requires_cmd = []

[api]
functions = ["help_{}"]
variables = []
# aliases = {{ top = "btop" }}
# completions = {{ "help_{}" = "_files" }}
"#,
        module_name, module_name, module_name
    );
    fs::write(target_dir.join("module.toml"), toml_content)?;

    let zsh_content = format!(
r#"# ============================================================
# {} 
# ============================================================

# Internal implementation (prefix with _{}_*)
_{}_help() {{
    echo "\033[1;36m:: {} Module\033[0m"
    echo "Edit this help text in init.zsh"
}}

# Public functions are mapped to internal ones here
# Do not use aliases for function mapping, use real functions:
help_{}() {{
    _{}_help "$@"
}}
"#,
        module_name, module_name, module_name, module_name, module_name, module_name
    );
    fs::write(target_dir.join("init.zsh"), zsh_content)?;

    println!("\n{} {}\n", "::".bold().cyan(), "Module Created".bold().cyan());
    println!("  {} {}", "name:".dimmed(), module_name.green());
    println!("  {} {}", "dir: ".dimmed(), target_dir_name.green());
    println!("  {} {}", "file:".dimmed(), "module.toml, init.zsh");
    println!("\n{} Edit the files, then run: {} {}\n", "=>".bold().blue(), "zrt reload".bold(), module_name.bold());

    Ok(())
}

fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
