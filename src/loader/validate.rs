use super::types::{DiscoveredModule, ModuleStatus};
use super::Loader;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

impl Loader {
    pub(crate) fn validate_commands(&self, modules: &mut [DiscoveredModule]) {
        for m in modules.iter_mut() {
            if m.status != ModuleStatus::Loaded {
                continue;
            }
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
                .map(|m| {
                    (
                        m.manifest.module.name.clone(),
                        m.manifest.module.version.clone(),
                    )
                })
                .collect();

            for m in modules.iter_mut() {
                if m.status != ModuleStatus::Loaded {
                    continue;
                }
                for dep in &m.manifest.module.deps {
                    match loaded.get(&dep.name) {
                        None => {
                            m.status =
                                ModuleStatus::SkippedMissingDep(dep.name.clone());
                            changed = true;
                            break;
                        }
                        Some(version) => {
                            if let Some(constraint) = &dep.version {
                                match satisfies(version, constraint) {
                                    Ok(true) => {}
                                    Ok(false) => {
                                        m.status = ModuleStatus::SkippedMissingDep(
                                            format!("{}@{}", dep.name, constraint),
                                        );
                                        changed = true;
                                        break;
                                    }
                                    Err(e) => {
                                        m.status =
                                            ModuleStatus::SkippedBadConstraint(format!(
                                                "dep '{}': {}",
                                                dep.name, e
                                            ));
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
                        "module '{}': completion function '{}' not found in any init.zsh \
                         (if it is defined in a sourced sub-file, this warning can be ignored)",
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

fn satisfies(version: &str, constraint: &str) -> Result<bool, String> {
    let version = parse_version(version)
        .ok_or_else(|| format!("invalid version string '{}'", version))?;

    if let Some(req) = constraint.strip_prefix(">=") {
        let req = parse_version(req.trim())
            .ok_or_else(|| format!("invalid version in constraint '{}'", constraint))?;
        return Ok(cmp_version(&version, &req) >= 0);
    }
    if let Some(req) = constraint.strip_prefix("<=") {
        let req = parse_version(req.trim())
            .ok_or_else(|| format!("invalid version in constraint '{}'", constraint))?;
        return Ok(cmp_version(&version, &req) <= 0);
    }
    if let Some(req) = constraint.strip_prefix('>') {
        let req = parse_version(req.trim())
            .ok_or_else(|| format!("invalid version in constraint '{}'", constraint))?;
        return Ok(cmp_version(&version, &req) > 0);
    }
    if let Some(req) = constraint.strip_prefix('<') {
        let req = parse_version(req.trim())
            .ok_or_else(|| format!("invalid version in constraint '{}'", constraint))?;
        return Ok(cmp_version(&version, &req) < 0);
    }
    if let Some(req) = constraint.strip_prefix('=') {
        let req = parse_version(req.trim())
            .ok_or_else(|| format!("invalid version in constraint '{}'", constraint))?;
        return Ok(cmp_version(&version, &req) == 0);
    }
    if let Some(req) = constraint.strip_prefix('~') {
        let req = parse_version(req.trim())
            .ok_or_else(|| format!("invalid version in constraint '{}'", constraint))?;
        return Ok(
            cmp_version(&version, &req) >= 0
                && version.get(0) == req.get(0)
                && version.get(1) == req.get(1),
        );
    }
    if let Some(req) = constraint.strip_prefix('^') {
        let req = parse_version(req.trim())
            .ok_or_else(|| format!("invalid version in constraint '{}'", constraint))?;
        return Ok(cmp_version(&version, &req) >= 0 && version.get(0) == req.get(0));
    }

    Err(format!(
        "unrecognised constraint '{}' (expected one of: =, >=, >, <=, <, ~, ^)",
        constraint
    ))
}

fn parse_version(s: &str) -> Option<Vec<u64>> {
    if s.is_empty() {
        return None;
    }
    s.split('.').map(|part| part.parse::<u64>().ok()).collect()
}

fn cmp_version(a: &[u64], b: &[u64]) -> i64 {
    let len = a.len().max(b.len());
    for i in 0..len {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        if av != bv {
            return if av > bv { 1 } else { -1 };
        }
    }
    0
}
