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
        let status = match &m.status {
            ModuleStatus::Loaded => "loaded ",
            ModuleStatus::SkippedMissingCmd(_)
            | ModuleStatus::SkippedMissingAnyCmd(_)
            | ModuleStatus::SkippedMissingDep(_) => "skipped",
            ModuleStatus::SkippedBadConstraint(_) => "error  ",
        };
        let dir_name = m.path.file_name().unwrap().to_string_lossy();
        input.push_str(&format!(
            "{:<20} {}  v{:<8} {}\n",
            m.manifest.module.name, status, m.manifest.module.version, dir_name,
        ));
    }

    let mut child = Command::new("fzf")
        .args([
            "--ansi",
            "--expect=ctrl-r",
            "--layout=reverse",
            "--border=rounded",
            "--header=  enter:info   ctrl-r:reload   esc:quit",
            "--preview=gaiety info {1}",
            "--preview-window=right:55%:wrap",
            "--bind=enter:execute(gaiety info {1})",
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
    let key = stdout.lines().next().unwrap_or("");

    if key == "ctrl-r" {
        println!("reload");
    }

    Ok(())
}
