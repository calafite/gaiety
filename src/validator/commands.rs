use crate::core::types::{DiscoveredModule, ModuleStatus};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn validate_commands(modules: &mut [DiscoveredModule]) {
    for m in modules.iter_mut() {
        if m.status != ModuleStatus::Loaded {
            continue;
        }
        for cmd in &m.manifest.module.requires_cmd {
            if !has_command(cmd) {
                m.status = ModuleStatus::SkippedMissingCmd(cmd.clone());
                break;
            }
        }
    }
}

pub fn validate_any_commands(modules: &mut [DiscoveredModule]) {
    for m in modules.iter_mut() {
        if m.status != ModuleStatus::Loaded {
            continue;
        }
        let cmds = &m.manifest.module.requires_any_cmd;
        if !cmds.is_empty() && !cmds.iter().any(|cmd| has_command(cmd)) {
            m.status = ModuleStatus::SkippedMissingAnyCmd(cmds.clone());
        }
    }
}

pub fn check_completions(modules: &[DiscoveredModule]) -> Vec<String> {
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

fn has_command(cmd: &str) -> bool {
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
    fn test_is_function_defined() {
        assert!(is_function_defined("my_func() {\n}", "my_func"));
        assert!(is_function_defined("my_func () {\n}", "my_func"));
        assert!(is_function_defined("function my_func {\n}", "my_func"));
        assert!(!is_function_defined("# my_func() {\n}", "my_func"));
        assert!(!is_function_defined("other_func() {\n}", "my_func"));
    }
}
