use super::types::{DiscoveredModule, ModuleStatus};
use super::Loader;
use crate::manifest::Manifest;
use anyhow::{Context, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fs;

impl Loader {
    pub(crate) fn discover_modules(&self) -> Result<Vec<DiscoveredModule>> {
        let mut modules = Vec::new();

        for (dir_index, dir) in self.dirs.iter().enumerate() {
            for entry in fs::read_dir(dir)
                .with_context(|| format!("Failed to read directory: {}", dir.display()))?
            {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    let toml_path = path.join("module.toml");
                    if toml_path.exists() {
                        let content = fs::read_to_string(&toml_path)?;
                        let manifest: Manifest = toml::from_str(&content).with_context(|| {
                            format!("Failed to parse manifest: {}", toml_path.display())
                        })?;

                        let dir_name = path.file_name().unwrap().to_string_lossy();
                        let prefix_order = dir_name
                            .split('_')
                            .next()
                            .and_then(|s| s.parse::<u32>().ok());

                        modules.push(DiscoveredModule {
                            path,
                            manifest,
                            prefix_order,
                            dir_index,
                            status: ModuleStatus::Loaded,
                        });
                    }
                }
            }
        }
 
        let mut seen: HashMap<String, ()> = HashMap::new();
        modules.reverse();
        modules.retain(|m| seen.insert(m.manifest.module.name.clone(), ()).is_none());
        modules.reverse();

        Ok(modules)
    }
 
    pub(crate) fn sort_modules(&self, modules: &mut Vec<DiscoveredModule>) {
        let n = modules.len();
        if n == 0 {
            return;
        }

        let name_to_idx: HashMap<&str, usize> = modules
            .iter()
            .enumerate()
            .map(|(i, m)| (m.manifest.module.name.as_str(), i))
            .collect();

        let mut in_degree = vec![0usize; n];
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

        for (i, m) in modules.iter().enumerate() {
            for dep in &m.manifest.module.deps {
                if let Some(&dep_idx) = name_to_idx.get(dep.name.as_str()) {
                    if !adj[dep_idx].contains(&i) {
                        adj[dep_idx].push(i);
                        in_degree[i] += 1;
                    }
                }
            }
        }

        type Key = Reverse<(usize, u32, String, usize)>;

        let make_key = |i: usize| -> Key {
            let m = &modules[i];
            Reverse((
                m.dir_index,
                m.prefix_order.unwrap_or(u32::MAX),
                m.manifest.module.name.clone(),
                i,
            ))
        };

        let mut heap: BinaryHeap<Key> = (0..n)
            .filter(|&i| in_degree[i] == 0)
            .map(make_key)
            .collect();

        let mut order: Vec<usize> = Vec::with_capacity(n);

        while let Some(Reverse((_, _, _, i))) = heap.pop() {
            order.push(i);
            for &j in &adj[i] {
                in_degree[j] -= 1;
                if in_degree[j] == 0 {
                    heap.push(make_key(j));
                }
            }
        }

        if order.len() < n {
            let seen: HashSet<usize> = order.iter().copied().collect();
            let mut tail: Vec<usize> = (0..n).filter(|i| !seen.contains(i)).collect();
            tail.sort_by(|&a, &b| {
                let (ma, mb) = (&modules[a], &modules[b]);
                ma.dir_index.cmp(&mb.dir_index).then_with(|| {
                    match (ma.prefix_order, mb.prefix_order) {
                        (Some(x), Some(y)) => x.cmp(&y),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => ma.manifest.module.name.cmp(&mb.manifest.module.name),
                    }
                })
            });
            order.extend(tail);
        }

        let mut slots: Vec<Option<DiscoveredModule>> =
            modules.drain(..).map(Some).collect();
        modules.extend(order.into_iter().map(|i| slots[i].take().unwrap()));
    }
}
