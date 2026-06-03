pub mod local;
pub mod remote;

use crate::core::types::DiscoveredModule;
use anyhow::Result;

pub trait ModuleSource {
    fn fetch_modules(&self) -> Result<Vec<DiscoveredModule>>;
}
