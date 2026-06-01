use crate::manifest::Manifest;
use std::path::PathBuf;

#[derive(Debug)]
pub struct DiscoveredModule {
    pub path: PathBuf,
    pub manifest: Manifest,
    pub prefix_order: Option<u32>,
    pub dir_index: usize,
    pub status: ModuleStatus,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ModuleStatus {
    Loaded,
    /// One of the `requires_cmd` binaries was not found in PATH.
    SkippedMissingCmd(String),
    /// None of the `requires_any_cmd` binaries were found in PATH.
    SkippedMissingAnyCmd(Vec<String>),
    /// A declared dependency was not loaded (missing or itself skipped).
    SkippedMissingDep(String),
    /// A declared dependency has a version constraint that could not be parsed.
    SkippedBadConstraint(String),
    /// This module is part of a dependency cycle and cannot be loaded.
    /// The Vec holds the cycle path, e.g. ["a", "b", "c", "a"].
    SkippedCycle(Vec<String>),
    /// The module.toml for this module could not be parsed.
    FailedManifest(String),
    /// A dep entry is listed more than once in module.toml.
    WarnDuplicateDep(String),
}

impl ModuleStatus {
    pub fn is_loaded(&self) -> bool {
        matches!(self, ModuleStatus::Loaded)
    }

    pub fn is_skipped(&self) -> bool {
        !self.is_loaded()
    }
}
