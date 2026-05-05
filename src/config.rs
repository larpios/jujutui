use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default = "default_theme_slug")]
    pub theme: String,
    #[serde(default = "default_transparent")]
    pub transparent_background: bool,
    #[serde(default)]
    pub colors: HashMap<String, String>,
}

fn default_theme_slug() -> String {
    "catppuccin-mocha".to_string()
}

fn default_transparent() -> bool {
    false
}

pub fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        });
    base.join("jujutui").join("config.toml")
}

pub fn load_config() -> Config {
    let path = config_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Config::default();
    };
    toml::from_str(&text).unwrap_or_default()
}
