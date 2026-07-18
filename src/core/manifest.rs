use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

const DEFAULT_DESCRIPTION: &str = "<manifest could not be parsed>";

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Manifest {
    pub module: ModuleMeta,
    #[serde(default)]
    pub api: ApiMeta,
    #[serde(default)]
    pub load: Option<LoadMeta>,
    #[serde(default)]
    pub source: Option<SourceMeta>,
}

impl Manifest {
    pub fn broken(dir_name: String) -> Self {
        Self {
            module: ModuleMeta {
                name: dir_name,
                description: Some(DEFAULT_DESCRIPTION.to_string()),
                version: "0.0.0".to_string(),
                deps: Vec::new(),
                tags: Vec::new(),
                requires_cmd: Vec::new(),
                requires_any_cmd: Vec::new(),
                implicit: None,
                enabled: default_enabled(),
            },
            api: ApiMeta::default(),
            load: None,
            source: None,
        }
    }

    pub fn load_mode(&self) -> LoadMode {
        self.load
            .as_ref()
            .map(|l| l.load_mode.clone())
            .unwrap_or(LoadMode::Eager)
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
    pub deps: Vec<Dependency>,
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
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum LoadMode {
    #[default]
    Eager,
    Lazy,
    Event,
}

impl<'de> Deserialize<'de> for LoadMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.trim().to_lowercase().as_str() {
            "eager" => Ok(LoadMode::Eager),
            "lazy" => Ok(LoadMode::Lazy),
            "event" => Ok(LoadMode::Event),
            _ => Err(serde::de::Error::custom(format!(
                "invalid load_mode: {}",
                s
            ))),
        }
    }
}

#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct LoadMeta {
    #[serde(default)]
    pub load_mode: LoadMode,
    #[serde(default)]
    pub events: Vec<String>,
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
}

//helper functions
const fn default_enabled() -> Option<bool> {
    Some(true)
}
