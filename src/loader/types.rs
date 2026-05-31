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
    SkippedMissingCmd(String),
    SkippedMissingAnyCmd(Vec<String>),
    SkippedMissingDep(String),
    SkippedBadConstraint(String),
}
