use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Settings {
    #[serde(default = "default_hold_threshold")]
    pub hold_threshold_ms: u64,
    #[serde(default = "default_true")]
    pub tap_to_toggle: bool,
}

fn default_hold_threshold() -> u64 {
    200
}
fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hold_threshold_ms: 200,
            tap_to_toggle: true,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Mappings {
    pub cursor: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub mappings: Mappings,
}

impl Config {
    pub fn load(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn default_config() -> Self {
        let mut cursor = HashMap::new();
        // ---- Cursor movement (ESDF) ----
        cursor.insert("e".into(), "Up".into());
        cursor.insert("s".into(), "Left".into());
        cursor.insert("d".into(), "Down".into());
        cursor.insert("f".into(), "Right".into());
        cursor.insert("w".into(), "Home".into());
        cursor.insert("r".into(), "End".into());
        // ---- Selection (direct keys, no modifier needed) ----
        cursor.insert("i".into(), "Shift+Up".into());
        cursor.insert("j".into(), "Shift+Left".into());
        cursor.insert("k".into(), "Shift+Down".into());
        cursor.insert("l".into(), "Shift+Right".into());
        cursor.insert("u".into(), "Shift+Home".into());
        cursor.insert("o".into(), "Shift+End".into());
        // ---- Delete ----
        cursor.insert("n".into(), "Backspace".into());
        cursor.insert("m".into(), "Delete".into());
        cursor.insert("y".into(), "Shift+End, Delete".into());
        cursor.insert("h".into(), "Shift+Home, Delete".into());
        cursor.insert("backslash".into(), "Home, Shift+End, Delete".into());
        // ---- Select whole ----
        cursor.insert("space".into(), "Home, Shift+End".into());
        // ---- Newline ----
        cursor.insert("alt+enter".into(), "Home, Enter, Up".into());
        cursor.insert("enter".into(), "End, Enter".into());

        Config {
            settings: Settings::default(),
            mappings: Mappings { cursor },
        }
    }

    pub fn write_default(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(path, Self::default_toml_content())?;
        Ok(())
    }

    /// Embedded default.toml content — shipped inside the binary, no external file needed.
    pub fn default_toml_content() -> &'static str {
        include_str!("../default.toml")
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub config: Config,
    pub config_path: PathBuf,
}

impl AppConfig {
    pub fn new(config: Config, config_path: PathBuf) -> Self {
        Self {
            config,
            config_path,
        }
    }

    pub fn reload(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.config = Config::load(&self.config_path).unwrap_or_else(|_| {
            log::warn!("Failed to reload config, keeping current");
            self.config.clone()
        });
        log::info!("Config reloaded");
        Ok(())
    }
}

pub type SharedConfig = std::sync::Arc<RwLock<AppConfig>>;
