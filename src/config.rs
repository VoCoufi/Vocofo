use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    #[serde(default)]
    pub connections: Vec<ConnectionProfile>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ConnectionProfile {
    pub name: String,
    pub protocol: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub key_path: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct GeneralConfig {
    pub show_hidden: bool,
    pub default_path: String,
    pub show_preview_on_start: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            connections: Vec::new(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            default_path: ".".to_string(),
            show_preview_on_start: false,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = config_file_path();
        match fs::read_to_string(&config_path) {
            Ok(content) => {
                toml::from_str(&content).unwrap_or_default()
            }
            Err(_) => Self::default(),
        }
    }
}

fn config_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("vocofo")
        .join("config.toml")
}
