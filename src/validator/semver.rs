use crate::core::parse_version_lenient;
use crate::core::types::{DiscoveredModule, ModuleStatus};
use anyhow::Result;

pub fn validate_dependencies(modules: &mut [DiscoveredModule]) {
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
                                    m.status = ModuleStatus::SkippedMissingDep(format!(
                                        "{}@{}",
                                        dep.name, constraint
                                    ));
                                    changed = true;
                                    break;
                                }
                                Err(e) => {
                                    m.status = ModuleStatus::SkippedBadConstraint(format!(
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

fn satisfies(version: &str, constraint: &str) -> Result<bool, String> {
    let v = parse_version_lenient(version)
        .map_err(|e| format!("invalid version '{}': {}", version, e))?;
    let req = semver::VersionReq::parse(constraint)
        .map_err(|e| format!("invalid constraint '{}': {}", constraint, e))?;
    Ok(req.matches(&v))
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
}
