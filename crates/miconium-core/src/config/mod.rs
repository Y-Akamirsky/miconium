use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to serialize config: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("no config found at default paths")]
    NotFound,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Config {
    #[serde(default)]
    pub pack: PackConfig,
    #[serde(default)]
    pub colors: ColorsConfig,
    #[serde(default)]
    pub export: ExportConfig,
    #[serde(default)]
    pub gui: GuiConfig,
    /// Name of the preset (file under `~/.config/miconium/presets/`) that the
    /// daemon should use by default, and that the GUI loads on startup.
    #[serde(default)]
    pub default_preset: Option<String>,
    /// Lifetime (in hours) of generated icon themes produced by the daemon.
    /// `None` falls back to the daemon default (72h); `0` disables auto-cleanup.
    #[serde(default)]
    pub cache_ttl_hours: Option<u64>,
    /// How often (seconds) the daemon polls the colour-source file of the
    /// default preset for changes. `None` falls back to
    /// [`DEFAULT_SOURCE_POLL_SECS`].
    #[serde(default)]
    pub source_poll_seconds: Option<u64>,
    /// How often (seconds) the daemon re-reads which preset is the default.
    /// `None` falls back to [`DEFAULT_PRESET_POLL_SECS`].
    #[serde(default)]
    pub preset_poll_seconds: Option<u64>,
}

/// The daemon default when `cache_ttl_hours` is unset.
pub const DEFAULT_CACHE_TTL_HOURS: u64 = 72;
/// The daemon default when `source_poll_seconds` is unset.
pub const DEFAULT_SOURCE_POLL_SECS: u64 = 15;
/// The daemon default when `preset_poll_seconds` is unset.
pub const DEFAULT_PRESET_POLL_SECS: u64 = 15 * 60;

impl Config {
    /// Resolve the effective cache TTL, falling back to
    /// [`DEFAULT_CACHE_TTL_HOURS`].
    #[must_use]
    pub fn cache_ttl_hours(&self) -> u64 {
        self.cache_ttl_hours.unwrap_or(DEFAULT_CACHE_TTL_HOURS)
    }

    /// Resolve the colour-source poll interval, falling back to
    /// [`DEFAULT_SOURCE_POLL_SECS`].
    #[must_use]
    pub fn source_poll_seconds(&self) -> u64 {
        self.source_poll_seconds.unwrap_or(DEFAULT_SOURCE_POLL_SECS)
    }

    /// Resolve the default-preset re-read interval, falling back to
    /// [`DEFAULT_PRESET_POLL_SECS`].
    #[must_use]
    pub fn preset_poll_seconds(&self) -> u64 {
        self.preset_poll_seconds.unwrap_or(DEFAULT_PRESET_POLL_SECS)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PackConfig {
    pub path: Option<String>,
    #[serde(default = "default_pack_name")]
    pub name: String,
    #[serde(default)]
    pub category_overrides: HashMap<String, HashMap<String, CategoryOverride>>,
    #[serde(default)]
    pub selected_variants: HashMap<String, String>,
}

fn default_pack_name() -> String {
    "yamis".into()
}

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            path: None,
            name: default_pack_name(),
            category_overrides: HashMap::new(),
            selected_variants: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RotationCenter {
    #[default]
    Center,
    Ul,
    Ur,
    Dl,
    Dr,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AccessoryConfig {
    pub name: String,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub rotation: f64,
    #[serde(default = "default_scale")]
    pub scale: f64,
    #[serde(default)]
    pub rotation_center: RotationCenter,
    #[serde(default)]
    pub scale_center: RotationCenter,
}

impl Default for AccessoryConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale: 1.0,
            rotation_center: RotationCenter::default(),
            scale_center: RotationCenter::default(),
        }
    }
}

impl AccessoryConfig {
    #[must_use]
    pub fn has_offset(&self) -> bool {
        #[allow(clippy::float_cmp)]
        {
            self.x != 0.0 || self.y != 0.0 || self.rotation != 0.0 || self.scale != 1.0
            || self.rotation_center != RotationCenter::Center
            || self.scale_center != RotationCenter::Center
        }
    }

    #[must_use]
    pub fn layer_transform(&self) -> crate::svg_engine::LayerTransform {
        crate::svg_engine::LayerTransform {
            dx: self.x,
            dy: self.y,
            rotation: self.rotation,
            scale: self.scale,
            rotation_center: self.rotation_center,
            scale_center: self.scale_center,
        }
    }

    /// The user-facing name of the current rotation center.
    #[must_use]
    pub fn rotation_center_label(&self) -> &str {
        match self.rotation_center {
            RotationCenter::Center => "Center",
            RotationCenter::Ul => "UL",
            RotationCenter::Ur => "UR",
            RotationCenter::Dl => "DL",
            RotationCenter::Dr => "DR",
        }
    }

    /// The user-facing name of the current scale center.
    #[must_use]
    pub fn scale_center_label(&self) -> &str {
        match self.scale_center {
            RotationCenter::Center => "Center",
            RotationCenter::Ul => "UL",
            RotationCenter::Ur => "UR",
            RotationCenter::Dl => "DL",
            RotationCenter::Dr => "DR",
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
    pub accessories: Vec<AccessoryConfig>,
    #[serde(default = "default_frame_source")]
    pub frame_source: String,
    #[serde(default)]
    pub selected_static_frame: Option<String>,
    #[serde(default = "default_scale")]
    pub frame_scale: f64,
    #[serde(default = "default_scale")]
    pub icon_scale: f64,
    #[serde(default = "default_scale")]
    pub acc_scale: f64,
}

fn default_true() -> bool { true }

fn default_frame_source() -> String { "colorizable".into() }

fn default_scale() -> f64 { 1.0 }

#[must_use]
pub fn default_category_override(_category: &str) -> CategoryOverride {
    CategoryOverride {
        show_frame: false,
        show_accessories: false,
        selected_frame: None,
        accessories: Vec::new(),
        frame_source: default_frame_source(),
        selected_static_frame: None,
        frame_scale: 1.0,
        icon_scale: 1.0,
        acc_scale: 1.0,
    }
}

impl PackConfig {
    #[must_use]
    pub fn variant_override(&self, category: &str, variant: &str) -> CategoryOverride {
        self.category_overrides
            .get(category)
            .and_then(|v| v.get(variant))
            .cloned()
            .unwrap_or_else(|| default_category_override(category))
    }

    #[must_use]
    pub fn selected_variant(&self, category: &str) -> String {
        self.selected_variants
            .get(category)
            .cloned()
            .unwrap_or_else(|| "scalable".into())
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ColorsConfig {
    pub scheme: Option<String>,
    pub matugen: Option<String>,
    pub manual: Option<String>,
    pub overrides: Option<ColorOverrides>,
    pub map: LayerMap,
}

/// Which palette role is injected into each SVG layer. `frame` maps onto the
/// background color, `sign` onto the foreground color and `accessory` onto the
/// accent color that the SVG engine substitutes into each layer.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LayerMap {
    pub frame: String,
    pub sign: String,
    pub accessory: String,
}

impl Default for LayerMap {
    fn default() -> Self {
        Self {
            frame: "background".into(),
            sign: "foreground".into(),
            accessory: "accent".into(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ColorOverrides {
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub accent: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ExportConfig {
    pub output: Option<String>,
    pub sizes: Vec<u32>,
    pub frame_scale: f64,
    pub icon_scale: f64,
    pub acc_scale: f64,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            output: None,
            sizes: vec![16, 24, 32, 48, 64, 96, 128, 256, 512],
            frame_scale: 1.0,
            icon_scale: 1.0,
            acc_scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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

/// The path `save` will write to: the first existing config file, otherwise
/// the user config default (`~/.config/miconium/config.toml`).
#[must_use]
pub fn save_config_path() -> PathBuf {
    find_config_path().unwrap_or_else(|| {
        let home = std::env::var("HOME").map_or_else(|_| std::env::temp_dir(), PathBuf::from);
        home.join(".config/miconium/config.toml")
    })
}

/// Best-effort atomic-ish save of `config` to `save_config_path()`.
/// Errors are propagated so the caller can surface them in the GUI.
pub fn save(config: &Config) -> Result<(), ConfigError> {
    let path = save_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let toml = toml::to_string_pretty(config)?;
    std::fs::write(&path, toml)?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Preset system (item 12)
//
// A preset is just a named `Config`, stored as a standalone TOML file under
// `~/.config/miconium/presets/<name>.toml`. The main `miconium.toml` only
// records which preset is the default.
// ─────────────────────────────────────────────────────────────────────────

/// The directory holding preset files, derived from the user's home.
#[must_use]
pub fn presets_dir() -> PathBuf {
    presets_dir_in(None)
}

fn presets_dir_in(home: Option<&Path>) -> PathBuf {
    let base = home
        .map(Path::to_path_buf)
        .or_else(|| std::env::var("HOME").ok().map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    base.join(".config/miconium/presets")
}

/// Directories searched (in priority order) for preset files.
///
/// 1. The user presets dir (`~/.config/miconium/presets`)
/// 2. The packaged system location (`/usr/share/miconium/presets`)
#[must_use]
pub fn preset_search_dirs() -> Vec<PathBuf> {
    vec![presets_dir(), PathBuf::from("/usr/share/miconium/presets")]
}

/// Resolve the on-disk path of a preset named `name`, searching the standard
/// preset directories (user first, then system). Returns `None` when no
/// matching `<name>.toml` exists.
#[must_use]
pub fn resolve_preset_path(name: &str) -> Option<PathBuf> {
    let file = format!("{}.toml", sanitize_name(name));
    for dir in preset_search_dirs() {
        let candidate = dir.join(&file);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Replace filesystem-hostile characters so a preset name can never escape the
/// presets directory (no slashes, no traversal).
fn sanitize_name(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() {
        s = "preset".into();
    }
    s
}

fn preset_path_in(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{}.toml", sanitize_name(name)))
}

/// List the names of all stored presets (sorted). Presets found in the user
/// directory take precedence over identically-named system presets.
#[must_use]
pub fn list_presets() -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut names = Vec::new();
    for dir in preset_search_dirs() {
        for name in list_presets_in(&dir) {
            if seen.insert(name.clone()) {
                names.push(name);
            }
        }
    }
    names.sort();
    names
}

pub(crate) fn list_presets_in(dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return names;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("toml")) {
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    names
}

/// Persist `config` as a preset with the given `name`.
pub fn save_preset(name: &str, config: &Config) -> Result<(), ConfigError> {
    save_preset_in(&presets_dir(), name, config)
}

pub(crate) fn save_preset_in(dir: &Path, name: &str, config: &Config) -> Result<(), ConfigError> {
    std::fs::create_dir_all(dir)?;
    let path = preset_path_in(dir, name);
    let toml = toml::to_string_pretty(config)?;
    std::fs::write(&path, toml)?;
    Ok(())
}

/// Load a previously saved preset by `name`, searching the user directory
/// first and falling back to the packaged system presets.
pub fn load_preset(name: &str) -> Result<Config, ConfigError> {
    let path = resolve_preset_path(name).ok_or(ConfigError::NotFound)?;
    let path_str = path.to_string_lossy();
    load(&path_str)
}

#[cfg(test)]
pub(crate) fn load_preset_in(dir: &Path, name: &str) -> Result<Config, ConfigError> {
    let path = preset_path_in(dir, name);
    if !path.is_file() {
        return Err(ConfigError::NotFound);
    }
    load(&path.to_string_lossy())
}

/// Remove a preset by `name`. Removing a non-existent preset is a no-op.
pub fn delete_preset(name: &str) -> Result<(), ConfigError> {
    delete_preset_in(&presets_dir(), name)
}

pub(crate) fn delete_preset_in(dir: &Path, name: &str) -> Result<(), ConfigError> {
    let path = preset_path_in(dir, name);
    if path.is_file() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

/// The name of the default preset (read from the main `miconium.toml`).
#[must_use]
pub fn default_preset() -> Option<String> {
    load_or_default().ok().and_then(|c| c.default_preset)
}

/// Record `name` as the default preset in the main `miconium.toml`.
pub fn set_default_preset(name: &str) -> Result<(), ConfigError> {
    let mut cfg = load_or_default().unwrap_or_default();
    cfg.default_preset = Some(name.to_string());
    save(&cfg)
}

#[cfg(test)]
mod tests;
