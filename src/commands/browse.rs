use crate::core::loader::Loader;
use crate::core::types::ModuleStatus;
use anyhow::{Context, Result, bail};
use std::io::Write;
use std::process::{Command, Stdio};

pub fn run(directories: String) -> Result<()> {
    Helper::require_fzf()?;

    let loader_context = || format!("Failed to initialize loader for: {}", directories);
    let loader = Loader::new(&directories).with_context(loader_context)?;

    let modules_context = || "Failed to retrieve modules".to_string();
    let modules = loader.get_modules().with_context(modules_context)?;

    if modules.is_empty() {
        bail!("no modules found");
    }

    let input = Helper::generate_fzf_input(&modules);
    let output = Helper::run_fzf(&input)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let key = lines.next().unwrap_or("");

    let enter_pressed = key == "enter";
    if enter_pressed {
        let selected = lines.next().unwrap_or("").trim();
        let selected_name = selected.split_whitespace().next();

        if let Some(name) = selected_name {
            let find_predicate = |discovered_module: &&crate::core::types::DiscoveredModule| {
                discovered_module.manifest.module.name == name
            };
            if let Some(module) = modules.iter().find(find_predicate) {
                let init_path = module.path.join("init.zsh");
                let path_exists = init_path.exists();
                if path_exists {
                    println!("{}\t{}", name, init_path.display());
                }
            }
        }
    }

    Ok(())
}

struct Helper;

impl Helper {
    fn require_fzf() -> Result<()> {
        let fzf_absent = Command::new("fzf")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_err();
        if fzf_absent {
            bail!("fzf not found in PATH — required for gai browse");
        }
        Ok(())
    }

    fn generate_fzf_input(modules: &[crate::core::types::DiscoveredModule]) -> String {
        let mut input = String::new();
        for module in modules {
            let status_colored = match &module.status {
                ModuleStatus::Loaded => "\x1b[32mloaded \x1b[0m",
                ModuleStatus::WarnDuplicateDep(_) => "\x1b[33mwarn   \x1b[0m",
                ModuleStatus::SkippedMissingCmd(_)
                | ModuleStatus::SkippedMissingAnyCmd(_)
                | ModuleStatus::SkippedMissingDep(_) => "\x1b[33mskipped\x1b[0m",
                ModuleStatus::SkippedCycle(_)
                | ModuleStatus::SkippedBadConstraint(_)
                | ModuleStatus::FailedManifest(_) => "\x1b[31merror  \x1b[0m",
            };

            let version = format!("v{}", module.manifest.module.version);
            let description = module.manifest.module.description.as_deref().unwrap_or("—");

            input.push_str(&format!(
                "{:<18}  {}  \x1b[2m{:<10}\x1b[0m  \x1b[2m{}\x1b[0m\n",
                module.manifest.module.name, status_colored, version, description,
            ));
        }
        input
    }

    fn run_fzf(input: &str) -> Result<std::process::Output> {
        let spawn_context = || "Failed to spawn fzf command".to_string();
        let mut child = Command::new("fzf")
            .args([
                "--ansi",
                "--expect=enter",
                "--height=100%",
                "--layout=reverse",
                "--border=none",
                "--info=inline",
                "--header=  \x1b[1menter\x1b[0m: reload module   \x1b[1mesc\x1b[0m: quit",
                "--preview=CLICOLOR_FORCE=1 gaiety info {1}",
                "--preview-window=right:60%:wrap:border-left",
                "--color=border:#555555,header:#888888,hl:#00d7af,hl+:#00d7af",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .with_context(spawn_context)?;

        {
            let stdin_take_error = || "Failed to open fzf stdin".to_string();
            let mut stdin = child.stdin.take().context(stdin_take_error())?;

            let write_error = || "Failed to write input to fzf stdin".to_string();
            stdin
                .write_all(input.as_bytes())
                .with_context(write_error)?;
        }

        let wait_error = || "Failed to wait on fzf subprocess output".to_string();
        let output = child.wait_with_output().with_context(wait_error)?;
        Ok(output)
    }
}
