use crate::loader::types::ModuleStatus;
use crate::loader::Loader;
use anyhow::{bail, Result};
use std::io::Write;
use std::process::{Command, Stdio};

pub fn run(dirs: String) -> Result<()> {
    if Command::new("fzf")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        bail!("fzf not found in PATH — required for gai browse");
    }

    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    if modules.is_empty() {
        bail!("no modules found");
    }

    let mut input = String::new();
    for m in &modules {
        let status_colored = match &m.status {
            ModuleStatus::Loaded => "\x1b[32mloaded \x1b[0m",
            ModuleStatus::SkippedMissingCmd(_)
            | ModuleStatus::SkippedMissingAnyCmd(_)
            | ModuleStatus::SkippedMissingDep(_) => "\x1b[33mskipped\x1b[0m",
            ModuleStatus::SkippedBadConstraint(_) => "\x1b[31merror  \x1b[0m",
        };

        let version = format!("v{}", m.manifest.module.version);
        let desc = m.manifest.module.description.as_deref().unwrap_or("—");
        input.push_str(&format!(
            "{:<18}  {}  \x1b[2m{:<10}\x1b[0m  \x1b[2m{}\x1b[0m\n",
            m.manifest.module.name, status_colored, version, desc,
        ));
    }

    let mut child = Command::new("fzf")
        .args([
            "--ansi",
            "--expect=enter",
            "--layout=reverse",
            "--border=rounded",
            "--height=~80%",
            "--min-height=12",
            "--info=inline",
            "--header=  \x1b[1menter\x1b[0m: reload module   \x1b[1mesc\x1b[0m: quit",
            "--preview=gaiety info {1}",
            "--preview-window=right:55%:wrap:border-left",
            "--color=border:#555555,header:#888888,hl:#00d7af,hl+:#00d7af",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    {
        let mut stdin = child.stdin.take().expect("failed to open fzf stdin");
        stdin.write_all(input.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let key = lines.next().unwrap_or("");

    if key == "enter" {
        let selected = lines.next().unwrap_or("").trim();
        let module_name = selected.split_whitespace().next().unwrap_or("");
        if let Some(m) = modules.iter().find(|m| m.manifest.module.name == module_name) {
            let init_path = m.path.join("init.zsh");
            if init_path.exists() {
                println!("source:{}:{}", module_name, init_path.display());
            }
        }
    }

    Ok(())
}
