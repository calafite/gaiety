use anyhow::{Result, bail};
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use crate::core::types::DiscoveredModule;

pub fn require_git() -> Result<()> {
    let git_absent = Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err();
    if git_absent {
        bail!("git not found in PATH; required for gai install");
    }
    Ok(())
}

pub fn name_valid(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(valid_fn)
}

pub fn next_prefix(
    modules: &[DiscoveredModule],
    write_index: Option<usize>,
    directory: &Path,
) -> u32 {
    let filter_by_directory = |module: &&DiscoveredModule| match write_index {
        Some(index) => module.dir_index == index,
        None => module.path.parent() == Some(directory),
    };
    let prefix_order = |module: &DiscoveredModule| module.prefix_order;

    modules
        .iter()
        .filter(filter_by_directory)
        .filter_map(prefix_order)
        .max()
        .unwrap_or(0)
        + 1
}

pub fn temporary_name(tmp_type: &str) -> String {
    format!(".gai_{}_tmp_{}", tmp_type, std::process::id())
}

pub fn valid_fn(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}
