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

            let loaded: std::collections::HashMap<String, String> = modules
                .iter()
                .filter(|m| m.status == ModuleStatus::Loaded)
                .map(|m| (m.manifest.module.name.clone(), m.manifest.module.version.clone()))
                .collect();

            for m in modules.iter_mut() {
                if m.status != ModuleStatus::Loaded {
                    continue;
                }
                for dep in &m.manifest.module.deps {
                    match loaded.get(&dep.name) {
                        None => {
                            m.status = ModuleStatus::SkippedMissingDep(dep.name.clone());
                            changed = true;
                            break;
                        }
                        Some(version) => {
                            if let Some(constraint) = &dep.version {
                                if !satisfies(version, constraint) {
                                    m.status = ModuleStatus::SkippedMissingDep(
                                        format!("{}@{}", dep.name, constraint)
                                    );
                                    changed = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
 
    pub fn check_completions(&self, modules: &[DiscoveredModule]) -> Vec<String> {
        let all_content: String = modules
            .iter()
            .filter(|m| m.status == ModuleStatus::Loaded)
            .filter_map(|m| {
                let init_path = m.path.join("init.zsh");
                std::fs::read_to_string(init_path).ok()
            })
            .collect();

        let mut warnings = Vec::new();
        for m in modules {
            if m.status != ModuleStatus::Loaded {
                continue;
            }
            for comp_fn in m.manifest.api.completions.values() {
                if !all_content.contains(comp_fn.as_str()) {
                    warnings.push(format!(
                        "module '{}': completion function '{}' not found in any init.zsh",
                        m.manifest.module.name, comp_fn
                    ));
                }
            }
        }
        warnings
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

fn satisfies(version: &str, constraint: &str) -> bool {
    if let Some(required) = constraint.strip_prefix(">=") {
        compare_versions(version, required.trim()) >= 0
    } else if let Some(required) = constraint.strip_prefix('=') {
        compare_versions(version, required.trim()) == 0
    } else {
        true
    }
}

fn compare_versions(a: &str, b: &str) -> i32 {
    let a_parts: Vec<u64> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let b_parts: Vec<u64> = b.split('.').filter_map(|s| s.parse().ok()).collect();
    let len = a_parts.len().max(b_parts.len());
    for i in 0..len {
        let av = a_parts.get(i).copied().unwrap_or(0);
        let bv = b_parts.get(i).copied().unwrap_or(0);
        if av != bv {
            return if av > bv { 1 } else { -1 };
        }
    }
    0
}
