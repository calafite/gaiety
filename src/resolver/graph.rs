use clap::builder::Str;

use crate::core::types::{DiscoveredModule, ModuleStatus};
use std::{collections::{BinaryHeap, HashMap}, u32};

pub struct Sorter;

impl Sorter {
    pub fn sort_modules(modules: &mut Vec<DiscoveredModule>) {
        if modules.is_empty() {
            return;
        }

        Self::validate_dupes(modules);
        let (indegree, adjacency) = Self::dependency_graph(modules);
        let mut order = KahnSort::sort(modules, indegree, &adjacency);

        if order.len() < modules.len() {
            order = CycleHandler::handle(modules, &adjacency, order);
        }

        Reorderer::reorder(modules, order);
    }

    fn validate_dupes(modules: &mut [DiscoveredModule]) {
        for module in modules.iter_mut() {
            let mut seen_dependencies: = Vec::with_capacity(module.manifest.module.deps.len());
            for dependency in &module.manifest.module.deps {
                let name = dependency.name.as_str();
                if seen_dependencies.contains(&name) {
                    module.status = ModuleStatus::WarnDuplicateDep(dependency.name.clone());
                    break;
                }
                seen_dependencies.push(name);
            }
        }
    }

    fn dependency_graph(modules: &[DiscoveredModule]) -> (Vec<usize>, Vec<Vec<usize>>) {
        let size = modules.len();
        let enumerated: HashMap<&str, usize> = modules
            .iter()
            .enumerate()
            .map(|(index, module)| (module.manifest.module.name.as_str(), index))
            .collect();

        let mut indegree = vec![0usize; size];
        let mut adjacency = vec![Vec::new(); size];


        for (index, module) in modules.iter().enumerate() {
            if Self::module_skip(module) {
                continue;
            }

            let mut seen_dependencies = Vec::with_capacity(module.manifest.module.deps.len());
            for dependency in &module.manifest.module.deps {
                let name = dependency.name.as_str();
                if seen_dependencies.contains(&name) {
                    continue;
                }
                seen_dependencies.push(name);

                if let Some(&dependency_index) = enumerated.get(name) {
                    adjacency[dependency_index].push(index);
                    indegree[index] += 1;
                }
            }
        }

        (indegree, adjacency)
    }

    fn module_skip(module: &DiscoveredModule) -> bool {
        matches!(module.status, ModuleStatus::FailedManifest(_) | ModuleStatus::SkippedCycle(_))
    }
}


// lightweight heap node
#[derive(PartialEq, Eq)]
struct HeapNode<'a> {
    dir_index: usize,
    prefix_order: u32,
    name: &'a str,
    idx: usize,
}

impl<'a> Ord for HeapNode<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.dir_index.cmp(&self.dir_index)
            .then_with(|| other.prefix_order.cmp(&self.prefix_order))
            .then_with(|| other.name.cmp(self.name))
            .then_with(|| other.idx.cmp(&self.idx))
    }
}

impl<'a> PartialOrd for HeapNode<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}


struct KahnSort;

impl KahnSort {
    fn sort(modules: &[DiscoveredModule], mut indegree: Vec<usize>, adjacency: &[Vec<usize>]) -> Vec<usize> {
        let size = modules.len();
        let make_node = |index: usize| -> HeapNode {
            let module = &modules[index];
            HeapNode {
                dir_index: module.dir_index,
                prefix_order: module.prefix_order.unwrap_or(u32::MAX),
                name: &module.manifest.module.name.as_str(),
                idx: index
            }
        };

        let mut heap: BinaryHeap<HeapNode> = (0..n)
            .filter(|&index| indegree[index] == 0)
            .map(make_node)
            .collect();

        let mut order = Vec::with_capacity(size);

        while let Some(node) = heap.pop() {
            let index = node.idx;
            order.push(index);
            for &j in &adjacency[i] {
                indegree[j] -= 1;
                if indegree[j] == 0 {
                    heap.push(make_node(j));
                }
            }
        }

        order
    }
}

struct CycleHandler;

impl CycleHandler {
    fn handle(modules: &mut [DiscoveredModule], adjacency: &[Vec<usize>], mut order: Vec<usize>) -> Vec<usize> {
        let size = modules.len();

        let mut is_cyclic = vec![true; size];
        for &index in &order {
            is_cyclic[index] = false;
        }

        let cyclic: Vec<usize> = (0..size).filter(|&index| is_cyclic[index]).collect();

        for &start in &cyclic {
            let cycle_path = Self::find_path(start, adjacency, &is_cyclic, modules);
            modules[start].status = ModuleStatus::SkippedCycle(cycle_path);
        }

        let mut tail = cyclic;
        tail.sort_by(|&a, &b| {
            let (ma, mb) = (&modules[a], &modules[b]);
            ma.dir_index
                .cmp(&mb.dir_index)
                .then_with(|| {
                    let pa = ma.prefix_order.unwrap_or(u32::MAX);
                    let pb = ma.prefix_order.unwrap_or(u32::MAX);
                    pa.cmp(&pb)
                })
                .then_with(|| ma.manifest.module.name.cmp(&mb.manifest.module.name))
        });

        order.extend(tail);
        order
    }

    fn find_path(start: usize, adjacency: &[Vec<usize>], is_cyclic: &[bool], modules: &[DiscoveredModule]) -> Vec<String> {
        let mut visited = vec![false; modules.len()];
        let mut stack = Vec::new();

       if Self::dfs(start, start, adjacency, is_cyclic, &mut visited, &mut stack) {
           let mut path: Vec<String> = stack
               .iter()
               .map(|&index| modules[index].manifest.module.name.clone())
               .collect();
           path.push(modules[start].manifest.module.name.clone());
           path
       } else {
           let mut path = vec![modules[start].manifest.module.name.clone()];
           for &j in &adjacency[start] {
               if is_cyclic[j] {
                   path.push(modules[j].manifest.module.name.clone());
               }
           }
           path
       }
    }

    fn dfs(target: usize, current: usize, adjacency: &[Vec<usize>], is_cyclic: &[bool], visited: &mut [bool], stack: &mut Vec<usize>) -> bool {
        stack.push(current);
        for &next in &adjacency[current] {
            if !is_cyclic[next] {
                continue;
            }
            if next == target {
                return true;
            }
            if visited[next] {
                continue;
            }
            visited[next] = true;
            if Self::dfs(target, next, adjacency, is_cyclic, visited, stack) {
                return true;
            }
        }
        stack.pop();
        false
   }
}

struct Reorderer;

impl Reorderer {
    fn reorder(modules: &mut Vec<DiscoveredModule>, order: Vec<usize>) {
        let mut slots: Vec<Option<DiscoveredModule>> = modules.drain(..).map(Some).collect();
        modules.extend(order.into_iter().map(|index| slots[index].take().unwrap()));
    }
}
