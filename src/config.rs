use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Panel layout orientation
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum PanelLayout {
    #[default]
    Auto,
    Horizontal,
    Vertical,
}

impl PanelLayout {
    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::Horizontal,
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Auto,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Auto => Self::Vertical,
            Self::Horizontal => Self::Auto,
            Self::Vertical => Self::Horizontal,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Horizontal => "Horizontal",
            Self::Vertical => "Vertical",
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
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
    #[serde(default)]
    pub panel_layout: PanelLayout,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            default_path: ".".to_string(),
            show_preview_on_start: false,
            panel_layout: PanelLayout::default(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        Self::load_from(&config_file_path())
    }

    pub fn load_from(path: &PathBuf) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = config_file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::other(format!("Failed to serialize config: {}", e)))?;
        let tmp_path = path.with_extension("toml.tmp");
        fs::write(&tmp_path, content)?;
        fs::rename(&tmp_path, &path)
    }
}

fn config_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("vocofo")
        .join("config.toml")
}
