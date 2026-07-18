use crate::core::types::{DiscoveredModule, ModuleStatus};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub struct CommandValidator;

impl CommandValidator {
    pub fn cmds(modules: &mut [DiscoveredModule]) {
        for module in modules.iter_mut() {
            if module.status != ModuleStatus::Loaded {
                continue;
            }
            for cmd in &module.manifest.module.requires_cmd {
                if !Self::has_cmd(cmd) {
                    module.status = ModuleStatus::SkippedMissingCmd(cmd.clone());
                    break;
                }
            }
        }
    }

    pub fn any_cmds(modules: &mut [DiscoveredModule]) {
        for module in modules.iter_mut() {
            if module.status != ModuleStatus::Loaded {
                continue;
            }
            let cmds = &module.manifest.module.requires_any_cmd;
            if !cmds.is_empty() && !cmds.iter().any(|cmd| Self::has_cmd(cmd)) {
                module.status = ModuleStatus::SkippedMissingAnyCmd(cmds.clone());
            }
        }
    }

    pub fn comps(modules: &[DiscoveredModule]) -> Vec<String> {
        let all_content: String = modules
            .iter()
            .filter(|module| module.status == ModuleStatus::Loaded)
            .filter_map(|module| {
                let init_path = module.path.join("init.zsh");
                std::fs::read_to_string(init_path).ok()
            })
            .collect();

        let mut warnings = Vec::new();
        for module in modules {
            if module.status != ModuleStatus::Loaded {
                continue;
            }
            for comp_fn in module.manifest.api.completions.values() {
                if !Self::is_fn(&all_content, comp_fn) {
                    warnings.push(format!(
                        "module '{}': completion function '{}' not found in any init.zsh \
                        (if it is defined in a sourced sub-file, this warning can be ignored)",
                        module.manifest.module.name, comp_fn
                    ));
                }
            }
        }
        warnings
    }

    fn has_cmd(cmd: &str) -> bool {
        if let Ok(paths) = std::env::var("PATH") {
            for path in paths.split(':') {
                let p = Path::new(path).join(cmd);
                if p.is_file()
                    && let Ok(meta) = p.metadata()
                    && meta.permissions().mode() & 0o111 != 0
                {
                    return true;
                }
            }
        }
        false
    }

    fn is_fn(content: &str, fn_name: &str) -> bool {
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
}

#[cfg(test)]
mod tests {
    use crate::validator::commands::CommandValidator;

    #[test]
    fn test_is_function_defined() {
        assert!(CommandValidator::is_fn("my_func() {\n}", "my_func"));
        assert!(CommandValidator::is_fn("my_func () {\n}", "my_func"));
        assert!(CommandValidator::is_fn("function my_func {\n}", "my_func"));
        assert!(!CommandValidator::is_fn("# my_func() {\n}", "my_func"));
        assert!(!CommandValidator::is_fn("other_func() {\n}", "my_func"));
    }
}
