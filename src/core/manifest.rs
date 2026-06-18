use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Manifest {
    pub module: ModuleMeta,
    #[serde(default)]
    pub api: ApiMeta,
    #[serde(default)]
    pub source: Option<SourceMeta>,
}

impl Manifest {
    pub fn broken(dir_name: String) -> Self {
        Self {
            module: ModuleMeta {
                name: dir_name,
                description: Some("<manifest could not be parsed>".to_string()),
                version: "0.0.0".to_string(),
                deps: Vec::new(),
                tags: Vec::new(),
                requires_cmd: Vec::new(),
                requires_any_cmd: Vec::new(),
                implicit: None,
                enabled: default_enabled(),
            },
            api: ApiMeta::default(),
            source: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct SourceMeta {
    pub url: String,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub pin: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct ModuleMeta {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    #[serde(default)]
    pub deps: Vec<Dep>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub requires_cmd: Vec<String>,
    #[serde(default)]
    pub requires_any_cmd: Vec<String>,
    #[serde(default)]
    pub implicit: Option<bool>,
    #[serde(default = "default_enabled")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct Dep {
    pub name: String,
    pub version: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct ApiMeta {
    #[serde(default)]
    pub functions: Vec<String>,
    #[serde(default)]
    pub variables: Vec<String>,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
    #[serde(default)]
    pub completions: HashMap<String, String>,
    #[serde(default)]
    pub defer_on_cmd: bool,
}

//helper functions
const fn default_enabled() -> Option<bool> {
    Some(true)
}
