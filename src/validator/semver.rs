use std::collections::HashMap;

use crate::core::parse::parse_version;
use crate::core::types::{DiscoveredModule, ModuleStatus};
use anyhow::Result;

pub struct DependencyValidator;

impl DependencyValidator {
    pub fn validate(modules: &mut [DiscoveredModule]) {
        let size = modules.len();
        let mut loaded_map = HashMap::with_capacity(size);
        let mut is_loaded = vec![false; size];

        for (index, module) in modules.iter().enumerate() {
            if module.status == ModuleStatus::Loaded {
                loaded_map.insert(
                    module.manifest.module.name.clone(),
                    (index, module.manifest.module.version.clone()),
                );
                is_loaded[index] = true;
            }
        }

        let mut dependents = vec![Vec::new(); size];
        for (index, module) in modules.iter().enumerate() {
            if !is_loaded[index] {
                continue;
            }
            for dependency in &module.manifest.module.deps {
                if let Some(&(dependency_index, _)) = loaded_map.get(dependency.name.as_str()) {
                    dependents[dependency_index].push(index);
                }
            }
        }

        let mut invalid_queue = Vec::new();

        for (index, module) in modules.iter_mut().enumerate() {
            if !is_loaded[index] {
                continue;
            }

            for dependency in &module.manifest.module.deps {
                match loaded_map.get(dependency.name.as_str()) {
                    Some((_, dependency_version)) => {
                        if let Some(constraint) = &dependency.version {
                            match Self::satisfies(dependency_version, constraint) {
                                Ok(true) => {}
                                Ok(false) => {
                                    module.status = ModuleStatus::SkippedMissingDep(format!(
                                        "{}@{}",
                                        dependency.name, constraint
                                    ));
                                    is_loaded[index] = false;
                                    invalid_queue.push(index);
                                    break;
                                }
                                Err(err) => {
                                    module.status = ModuleStatus::SkippedBadConstraint(format!(
                                        "dep '{}': {}",
                                        dependency.name, err
                                    ));
                                    is_loaded[index] = false;
                                    invalid_queue.push(index);
                                    break;
                                }
                            }
                        }
                    }
                    None => {
                        module.status = ModuleStatus::SkippedMissingDep(dependency.name.clone());
                        is_loaded[index] = false;
                        invalid_queue.push(index);
                        break;
                    }
                }
            }
        }

        let mut head = 0;
        while head < invalid_queue.len() {
            let u = invalid_queue[head];
            head += 1;

            let name = modules[u].manifest.module.name.clone();

            for &v in &dependents[u] {
                if is_loaded[v] {
                    is_loaded[v] = false;
                    modules[v].status = ModuleStatus::SkippedMissingDep(name.clone());
                    invalid_queue.push(v);
                }
            }
        }
    }

    fn satisfies(version: &str, constraint: &str) -> Result<bool, String> {
        let version = parse_version(version)
            .map_err(|err| format!("invalid version '{}': {}", version, err))?;
        let required = semver::VersionReq::parse(constraint)
            .map_err(|err| format!("invalid constraint '{}': {}", constraint, err))?;
        Ok(required.matches(&version))
    }
}
