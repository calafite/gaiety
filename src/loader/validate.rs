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
                            m.status = ModuleStatus::SkippedMissingDep(dep.name.clone());
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
                if !is_function_defined(&all_content, comp_fn) {
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
    let v = super::parse_version_lenient(version)
        .map_err(|e| format!("invalid version '{}': {}", version, e))?;
    let req = semver::VersionReq::parse(constraint)
        .map_err(|e| format!("invalid constraint '{}': {}", constraint, e))?;
    Ok(req.matches(&v))
}

fn is_function_defined(content: &str, fn_name: &str) -> bool {
    let posix_paren = format!("{}(", fn_name);
    let posix_space = format!("{} (", fn_name);
    let keyword_form = format!("function {}", fn_name);
    content.lines().any(|line| {
        let t = line.trim_start();
        !t.starts_with('#')
            && (t.starts_with(&posix_paren)
                || t.starts_with(&posix_space)
                || t.starts_with(&keyword_form))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_satisfies() {
        assert!(satisfies("1.2.3", ">=1.0.0").unwrap());
        assert!(!satisfies("0.9.0", ">=1.0.0").unwrap());
        assert!(satisfies("2.0.0", "^2.0.0").unwrap());
        assert!(satisfies("1.5", "<2.0.0").unwrap());
    }

    #[test]
    fn test_is_function_defined() {
        assert!(is_function_defined("my_func() {\n}", "my_func"));
        assert!(is_function_defined("my_func () {\n}", "my_func"));
        assert!(is_function_defined("function my_func {\n}", "my_func"));
        assert!(!is_function_defined("# my_func() {\n}", "my_func"));
        assert!(!is_function_defined("other_func() {\n}", "my_func"));
    }
}
