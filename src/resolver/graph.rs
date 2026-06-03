use crate::core::types::{DiscoveredModule, ModuleStatus};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

pub fn sort_modules(modules: &mut Vec<DiscoveredModule>) {
    let n = modules.len();
    if n == 0 {
        return;
    }

    for m in modules.iter_mut() {
        let mut seen_deps: HashSet<&str> = HashSet::new();
        for dep in &m.manifest.module.deps {
            if !seen_deps.insert(dep.name.as_str()) {
                m.status = ModuleStatus::WarnDuplicateDep(dep.name.clone());
                break;
            }
        }
    }

    let name_to_idx: HashMap<&str, usize> = modules
        .iter()
        .enumerate()
        .map(|(i, m)| (m.manifest.module.name.as_str(), i))
        .collect();

    let mut in_degree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (i, m) in modules.iter().enumerate() {
        if matches!(
            m.status,
            ModuleStatus::FailedManifest(_) | ModuleStatus::SkippedCycle(_)
        ) {
            continue;
        }

        let mut seen_deps: HashSet<&str> = HashSet::new();
        for dep in &m.manifest.module.deps {
            if !seen_deps.insert(dep.name.as_str()) {
                continue;
            }
            if let Some(&dep_idx) = name_to_idx.get(dep.name.as_str()) {
                adj[dep_idx].push(i);
                in_degree[i] += 1;
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
        let scheduled: HashSet<usize> = order.iter().copied().collect();
        let cyclic: Vec<usize> = (0..n).filter(|i| !scheduled.contains(i)).collect();

        let cyclic_set: HashSet<usize> = cyclic.iter().copied().collect();

        for &start in &cyclic {
            let cycle_path = find_cycle_path(start, &adj, &cyclic_set, modules);
            modules[start].status = ModuleStatus::SkippedCycle(cycle_path);
        }

        let mut tail = cyclic;
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

fn find_cycle_path(
    start: usize,
    adj: &[Vec<usize>],
    cyclic_set: &HashSet<usize>,
    modules: &[DiscoveredModule],
) -> Vec<String> {
    let mut visited: HashSet<usize> = HashSet::new();
    let mut stack: Vec<usize> = Vec::new();

    if dfs_cycle(start, start, adj, cyclic_set, &mut visited, &mut stack) {
        let mut path: Vec<String> = stack
            .iter()
            .map(|&i| modules[i].manifest.module.name.clone())
            .collect();
        path.push(modules[start].manifest.module.name.clone());
        path
    } else {
        let mut path = vec![modules[start].manifest.module.name.clone()];
        for &j in &adj[start] {
            if cyclic_set.contains(&j) {
                path.push(modules[j].manifest.module.name.clone());
            }
        }
        path
    }
}

fn dfs_cycle(
    target: usize,
    current: usize,
    adj: &[Vec<usize>],
    cyclic_set: &HashSet<usize>,
    visited: &mut HashSet<usize>,
    stack: &mut Vec<usize>,
) -> bool {
    stack.push(current);
    for &next in &adj[current] {
        if !cyclic_set.contains(&next) {
            continue;
        }
        if next == target {
            return true;
        }
        if visited.contains(&next) {
            continue;
        }
        visited.insert(next);
        if dfs_cycle(target, next, adj, cyclic_set, visited, stack) {
            return true;
        }
    }
    stack.pop();
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::local::LocalSource;
    use crate::sources::ModuleSource;
    use std::fs;
    use std::path::PathBuf;

    fn create_temp_dir(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("gai_test_resolver_{}_{}", name, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros()));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn test_discover_and_sort() {
        let temp = create_temp_dir("sort");
        let m1_dir = temp.join("01_m1");
        let m2_dir = temp.join("02_m2");
        fs::create_dir_all(&m1_dir).unwrap();
        fs::create_dir_all(&m2_dir).unwrap();

        fs::write(m1_dir.join("module.toml"), r#"
[module]
name = "m1"
version = "1.0.0"
deps = [ { name = "m2" } ]
"#).unwrap();

        fs::write(m2_dir.join("module.toml"), r#"
[module]
name = "m2"
version = "2.0.0"
"#).unwrap();

        let local_source = LocalSource::new(vec![temp.clone()]);
        let mut modules = local_source.fetch_modules().unwrap();
        assert_eq!(modules.len(), 2);

        sort_modules(&mut modules);
        assert_eq!(modules[0].manifest.module.name, "m2");
        assert_eq!(modules[1].manifest.module.name, "m1");

        let _ = fs::remove_dir_all(&temp);
    }
}
