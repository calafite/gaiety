use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub module: ModuleMeta,
    #[serde(default)]
    pub api: ApiMeta,
}

#[derive(Debug, Deserialize)]
pub struct ModuleMeta {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    #[serde(default)]
    pub deps: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub requires_cmd: Vec<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ApiMeta {
    #[serde(default)]
    pub functions: Vec<String>,
    #[serde(default)]
    pub variables: Vec<String>,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
    #[serde(default)]
    pub completions: HashMap<String, String>,
}
