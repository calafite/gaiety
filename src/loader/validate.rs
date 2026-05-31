use super::types::{DiscoveredModule, ModuleStatus};
use super::Loader;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

impl Loader {
    pub(crate) fn validate_commands(&self, modules: &mut [DiscoveredModule]) {
        for m in modules.iter_mut() {
            for cmd in &m.manifest.module.requires_cmd {
                if !self.has_command(cmd) {
                    m.status = ModuleStatus::SkippedMissingCmd(cmd.clone());
                    break;
                }
            }
        }
    }

    pub(crate) fn validate_any_commands(&self, modules: &mut [DiscoveredModule]) {
        for m in modules.iter_mut() {
            if m.status != ModuleStatus::Loaded {
                continue;
            }
            let cmds = &m.manifest.module.requires_any_cmd;
            if !cmds.is_empty() && !cmds.iter().any(|cmd| self.has_command(cmd)) {
                m.status = ModuleStatus::SkippedMissingAnyCmd(cmds.clone());
            }
        }
    }

    pub(crate) fn validate_dependencies(&self, modules: &mut [DiscoveredModule]) {
        let mut changed = true;
        while changed {
            changed = false;
            
            let loaded_names: std::collections::HashSet<&str> = modules
                .iter()
                .filter(|m| m.status == ModuleStatus::Loaded)
                .map(|m| m.manifest.module.name.as_str())
                .collect();

            for m in modules.iter_mut() {
                if m.status == ModuleStatus::Loaded {
                    for dep in &m.manifest.module.deps {
                        if !loaded_names.contains(dep.as_str()) {
                            m.status = ModuleStatus::SkippedMissingDep(dep.clone());
                            changed = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    fn has_command(&self, cmd: &str) -> bool {
        if let Ok(paths) = std::env::var("PATH") {
            for path in paths.split(':') {
                let p = Path::new(path).join(cmd);
                if p.is_file() {
                    if let Ok(meta) = p.metadata() {
                        if meta.permissions().mode() & 0o111 != 0 {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

