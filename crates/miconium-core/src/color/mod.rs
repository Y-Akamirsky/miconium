use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::{ColorsConfig, LayerMap};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ColorError {
    #[error("failed to parse color: {0}")]
    Parse(String),
    #[error("failed to read file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("failed to parse TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("no color source configured")]
    NoSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub foreground: String,
    pub background: String,
    pub accent: String,
    /// Named color roles collected from the source (e.g. all Material roles
    /// from a matugen palette, or the KDE colour-scheme groups).
    pub roles: HashMap<String, String>,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            foreground: "#ffffff".into(),
            background: "#1e1e2e".into(),
            accent: "#89b4fa".into(),
            roles: HashMap::new(),
        }
    }
}

impl Palette {
    /// Look up a color by role name. The three summary slots
    /// (`foreground`, `background`, `accent`) plus the alias `primary` are
    /// served directly; any other role is read from the `roles` map.
    #[must_use]
    pub fn get(&self, role: &str) -> Option<&str> {
        match role {
            "foreground" => Some(&self.foreground),
            "background" => Some(&self.background),
            "accent" | "primary" => Some(&self.accent),
            other => self.roles.get(other).map(String::as_str),
        }
    }

    /// Produce a derived palette where each layer maps to a chosen role.
    /// `frame` becomes `background`, `sign` becomes `foreground` and
    /// `accessory` becomes `accent` (the colors the SVG engine injects into
    /// each layer). Unmatched roles leave the slot at its current value.
    #[must_use]
    pub fn with_layer_map(&self, map: &LayerMap) -> Self {
        let mut p = self.clone();
        if let Some(c) = self.get(&map.frame) {
            p.background = c.to_string();
        }
        if let Some(c) = self.get(&map.sign) {
            p.foreground = c.to_string();
        }
        if let Some(c) = self.get(&map.accessory) {
            p.accent = c.to_string();
        }
        p
    }

    /// The distinct role names available for layer mapping.
    #[must_use]
    pub fn role_names(&self) -> Vec<String> {
        let mut names = vec![
            "foreground".to_string(),
            "background".to_string(),
            "accent".to_string(),
        ];
        let mut extra: Vec<String> = self
            .roles
            .keys()
            .filter(|k| {
                *k != "foreground" && *k != "background" && *k != "accent" && *k != "primary"
            })
            .cloned()
            .collect();
        extra.sort();
        names.extend(extra);
        names
    }

    #[must_use]
    pub fn apply_overrides(mut self, overrides: &crate::config::ColorOverrides) -> Self {
        if let Some(c) = &overrides.foreground {
            self.foreground.clone_from(c);
        }
        if let Some(c) = &overrides.background {
            self.background.clone_from(c);
        }
        if let Some(c) = &overrides.accent {
            self.accent.clone_from(c);
        }
        self
    }
}

pub enum ColorSource {
    XdgColors(String),
    MatugenJson(String),
    ManualToml(String),
}

impl ColorSource {
    #[must_use]
    pub fn from_config(config: &ColorsConfig) -> Option<Self> {
        if let Some(path) = &config.scheme {
            return Some(Self::XdgColors(path.clone()));
        }
        if let Some(path) = &config.matugen {
            return Some(Self::MatugenJson(path.clone()));
        }
        if let Some(path) = &config.manual {
            return Some(Self::ManualToml(path.clone()));
        }
        None
    }

    pub fn resolve(&self) -> Result<Palette, ColorError> {
        match self {
            Self::XdgColors(path) => parse_colors_file(path),
            Self::MatugenJson(path) => parse_matugen_json(path),
            Self::ManualToml(path) => parse_manual_toml(path),
        }
    }
}

pub fn resolve_palette(config: &ColorsConfig) -> Result<Palette, ColorError> {
    let source = ColorSource::from_config(config).ok_or(ColorError::NoSource)?;
    let palette = source.resolve()?;

    Ok(match &config.overrides {
        Some(o) => palette.apply_overrides(o),
        None => palette,
    })
}

/// The kind of colour source selectable in the GUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSourceKind {
    Xdg,
    Matugen,
    Manual,
}

impl ColorSourceKind {
    /// The typical extension of files for this kind.
    fn expected_ext(&self) -> &str {
        match self {
            Self::Matugen => "json",
            Self::Manual => "toml",
            Self::Xdg => "colors",
        }
    }

    /// Well-known locations probed first (exact filenames), relative to `root`.
    fn fixed_candidates_in(self, root: &Path) -> Vec<PathBuf> {
        let join = |p: &str| root.join(p);
        match self {
            Self::Matugen => vec![
                join(".config/matugen/colors.json"),
                join(".config/matugen/dms-colors.json"),
                join(".cache/matugen/colors.json"),
                join(".config/dank/.dms/dms-colors.json"),
                join(".config/dms/dms-colors.json"),
            ],
            Self::Xdg => vec![join(".config/kdeglobals")],
            Self::Manual => Vec::new(),
        }
    }

    /// Directories scanned (flat) for a matching file when no fixed candidate
    /// exists.
    fn scan_dirs(self) -> &'static [&'static str] {
        match self {
            Self::Matugen => &[".config/matugen", ".cache/matugen", ".config/dank/.dms", ".config/dms"],
            Self::Xdg | Self::Manual => &[".config"],
        }
    }

    /// Recursive scan helper for a directory.
    fn scan_dir(dir: &Path, ext: &str, depth: usize) -> Option<PathBuf> {
        if depth == 0 {
            return None;
        }
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file()
                && p.extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case(ext))
            {
                return Some(p);
            }
            if p.is_dir() {
                if let Some(found) = Self::scan_dir(&p, ext, depth - 1) {
                    return Some(found);
                }
            }
        }
        None
    }
}

/// Probe a root directory for a colour source of the given kind, returning the
/// most likely file. Internal core of [`probe_source_path`], factored out so
/// tests can drive it with a synthetic home.
fn probe_in_root(root: &Path, kind: ColorSourceKind) -> Option<PathBuf> {
    for c in kind.fixed_candidates_in(root) {
        if c.is_file() {
            return Some(c);
        }
    }
    for d in kind.scan_dirs() {
        let dir = root.join(d);
        if let Some(found) = ColorSourceKind::scan_dir(&dir, kind.expected_ext(), 2) {
            return Some(found);
        }
    }
    None
}

/// Probe common filesystem locations (dms/matugen, XDG) for a colour source of
/// the given kind, returning the first file found. Used to pre-fill the path
/// entry when nothing is configured yet.
#[must_use]
pub fn probe_source_path(kind: ColorSourceKind) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    probe_in_root(Path::new(&home), kind)
}

pub fn parse_colors_file(path: &str) -> Result<Palette, ColorError> {
    let content = std::fs::read_to_string(path)?;
    parse_xdg_colors(&content)
}

pub fn parse_xdg_colors(content: &str) -> Result<Palette, ColorError> {
    let mut palette = Palette::default();
    let mut section = String::new();
    let mut kde: HashMap<String, String> = HashMap::new();
    let mut has_kde = false;

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(sec) = line.strip_prefix('[') {
            section = sec.split(']').next().unwrap_or_default().to_string();
            continue;
        }
        if line.starts_with('#') || line.starts_with(';') || line.starts_with('=') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            if let Some(hex) = parse_kde_rgb(value) {
                has_kde = true;
                kde.insert(format!("{section}/{key}"), hex);
            }
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let hex = value.trim().trim_start_matches('#');
            if !is_hex_color(hex) {
                continue;
            }
            let hex = format!("#{hex}");
            match key {
                "foreground" | "Foreground" => palette.foreground = hex,
                "background" | "Background" => palette.background = hex,
                "accent" | "Accent" | "color0" => palette.accent = hex,
                _ => {}
            }
        }
    }

    if has_kde {
        parse_kde_roles(&mut palette, &kde);
    }

    Ok(palette)
}

fn parse_kde_roles(palette: &mut Palette, kde: &HashMap<String, String>) {
    let lookup = |group: &str, key: &str| kde.get(&format!("{group}/{key}"));

    if let Some(bg) = lookup("Colors:Window", "BackgroundNormal") {
        palette.background.clone_from(bg);
        palette.roles.entry("background".into()).or_insert_with(|| bg.clone());
        palette.roles.entry("surface".into()).or_insert_with(|| bg.clone());
    }
    if let Some(bg) = lookup("Colors:View", "BackgroundNormal") {
        palette.roles.entry("surface".into()).or_insert_with(|| bg.clone());
    }
    if let Some(fg) = lookup("Colors:Window", "ForegroundNormal") {
        palette.foreground.clone_from(fg);
        palette.roles.entry("foreground".into()).or_insert_with(|| fg.clone());
        palette.roles.entry("on_surface".into()).or_insert_with(|| fg.clone());
    }
    if let Some(ac) = lookup("Colors:Selection", "BackgroundNormal") {
        palette.accent.clone_from(ac);
        palette.roles.entry("primary".into()).or_insert_with(|| ac.clone());
    }
    if let Some(fg) = lookup("Colors:View", "ForegroundNormal") {
        palette.roles.entry("foreground".into()).or_insert_with(|| fg.clone());
        palette.roles.entry("on_surface".into()).or_insert_with(|| fg.clone());
    }
}

fn parse_kde_rgb(value: &str) -> Option<String> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return None;
    }
    let r: u8 = parts[0].parse().ok()?;
    let g: u8 = parts[1].parse().ok()?;
    let b: u8 = parts[2].parse().ok()?;
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

fn is_hex_color(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|c| c.is_ascii_hexdigit())
}

pub fn parse_matugen_json(path: &str) -> Result<Palette, ColorError> {
    let content = std::fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;

    let mut palette = Palette::default();

    if let Some(colors) = json.get("colors").and_then(|v| v.as_object()) {
        let scheme = colors
            .get("dark")
            .or_else(|| colors.get("light"))
            .and_then(|v| v.as_object());
        if let Some(scheme) = scheme {
            collect_roles(&mut palette, scheme);
        } else {
            collect_roles(&mut palette, colors);
        }
    } else if let Some(json) = json.as_object() {
        // Standalone flat matugen output (colors at top level).
        collect_roles(&mut palette, json);
    }

    if let Some(bg) = palette.roles.get("background").or_else(|| palette.roles.get("surface")) {
        palette.background.clone_from(bg);
    }
    if let Some(fg) = palette
        .roles
        .get("on_background")
        .or_else(|| palette.roles.get("on_surface"))
    {
        palette.foreground.clone_from(fg);
    }
    if let Some(ac) = palette.roles.get("primary") {
        palette.accent.clone_from(ac);
    }

    Ok(palette)
}

fn collect_roles(palette: &mut Palette, colors: &serde_json::Map<String, serde_json::Value>) {
    for (key, value) in colors {
        let Some(s) = value.as_str() else {
            continue;
        };
        let s = s.trim_start_matches('#');
        if !is_hex_color(s) {
            continue;
        }
        palette.roles.insert(key.clone(), format!("#{s}"));
    }
}

#[derive(serde::Deserialize)]
struct ManualColorToml {
    manual: ManualColorSection,
}

#[derive(serde::Deserialize)]
struct ManualColorSection {
    bottom: Option<String>,
    top: Option<String>,
}

pub fn parse_manual_toml(path: &str) -> Result<Palette, ColorError> {
    let content = std::fs::read_to_string(path)?;
    let parsed: ManualColorToml = toml::from_str(&content)?;

    let mut palette = Palette::default();

    if let Some(top) = &parsed.manual.top {
        palette.accent.clone_from(top);
    }
    if let Some(bottom) = &parsed.manual.bottom {
        palette.background.clone_from(bottom);
    }

    Ok(palette)
}

#[cfg(test)]
mod tests;
