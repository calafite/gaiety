use anyhow::{Result, bail};
use std::process::Command;
use std::process::Stdio;

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

pub fn temporary_name(tmp_type: &str) -> String {
    format!(".gai_{}_tmp_{}", tmp_type, std::process::id())
}

pub fn valid_fn(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}
