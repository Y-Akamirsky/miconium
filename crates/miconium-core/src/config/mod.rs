use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("no config found at default paths")]
    NotFound,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub pack: PackConfig,
    #[serde(default)]
    pub colors: ColorsConfig,
    #[serde(default)]
    pub export: ExportConfig,
    #[serde(default)]
    pub gui: GuiConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PackConfig {
    pub path: Option<String>,
    #[serde(default = "default_pack_name")]
    pub name: String,
    #[serde(default)]
    pub category_overrides: HashMap<String, CategoryOverride>,
}

fn default_pack_name() -> String {
    "mono".into()
}

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            path: None,
            name: default_pack_name(),
            category_overrides: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CategoryOverride {
    #[serde(default = "default_true")]
    pub show_frame: bool,
    #[serde(default = "default_true")]
    pub show_accessories: bool,
    #[serde(default)]
    pub selected_frame: Option<String>,
    #[serde(default)]
    pub selected_accessories: Vec<String>,
    #[serde(default = "default_frame_source")]
    pub frame_source: String,
    #[serde(default)]
    pub selected_static_frame: Option<String>,
}

fn default_true() -> bool { true }

fn default_frame_source() -> String { "colorizable".into() }

#[must_use]
pub fn default_category_override(category: &str) -> CategoryOverride {
    CategoryOverride {
        show_frame: !matches!(category, "devices" | "emblems" | "mime" | "places"),
        show_accessories: !matches!(category, "devices" | "emblems" | "mime" | "places"),
        selected_frame: None,
        selected_accessories: Vec::new(),
        frame_source: default_frame_source(),
        selected_static_frame: None,
    }
}

impl PackConfig {
    #[must_use]
    pub fn category_override(&self, category: &str) -> CategoryOverride {
        self.category_overrides
            .get(category)
            .cloned()
            .unwrap_or_else(|| default_category_override(category))
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct ColorsConfig {
    pub scheme: Option<String>,
    pub matugen: Option<String>,
    pub manual: Option<String>,
    pub overrides: Option<ColorOverrides>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ColorOverrides {
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub accent: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct ExportConfig {
    pub output: Option<String>,
    pub sizes: Vec<u32>,
    pub generate_16_symlinks: bool,
    pub frame_scale: f64,
    pub icon_scale: f64,
    pub acc_scale: f64,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            output: None,
            sizes: vec![16, 24, 32, 48, 64, 96, 128, 256, 512],
            generate_16_symlinks: true,
            frame_scale: 1.0,
            icon_scale: 1.0,
            acc_scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct GuiConfig {
    pub window_width: i32,
    pub window_height: i32,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            window_width: 800,
            window_height: 600,
        }
    }
}

const CONFIG_FILE_NAME: &str = "miconium.toml";

pub(crate) fn config_paths_for(home: Option<&std::path::Path>) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join(CONFIG_FILE_NAME));
    }

    let home_dir = home.map_or_else(
        || std::env::var("HOME").ok().map(std::path::PathBuf::from),
        |h| Some(h.to_path_buf()),
    );

    if let Some(home) = home_dir {
        paths.push(home.join(".config/miconium/config.toml"));
        paths.push(home.join(".config/miconium.toml"));
        paths.push(home.join(".miconium.toml"));
    }

    paths
}

pub(crate) fn default_config_paths() -> Vec<PathBuf> {
    config_paths_for(None)
}

pub fn load(path: &str) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

pub fn load_or_default() -> Result<Config, ConfigError> {
    for path in default_config_paths() {
        if path.is_file() {
            return load(&path.to_string_lossy());
        }
    }

    Ok(Config::default())
}

#[must_use]
pub fn find_config_path() -> Option<PathBuf> {
    default_config_paths().into_iter().find(|p| p.is_file())
}

#[cfg(test)]
mod tests;
