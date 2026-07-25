use crate::config::ColorsConfig;
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
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            foreground: "#ffffff".into(),
            background: "#1e1e2e".into(),
            accent: "#89b4fa".into(),
        }
    }
}

impl Palette {
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

pub fn parse_colors_file(path: &str) -> Result<Palette, ColorError> {
    let content = std::fs::read_to_string(path)?;
    parse_xdg_colors(&content)
}

pub fn parse_xdg_colors(content: &str) -> Result<Palette, ColorError> {
    let mut palette = Palette::default();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "foreground" | "Foreground" => palette.foreground = value.to_string(),
                "background" | "Background" => palette.background = value.to_string(),
                "accent" | "Accent" | "color0" => palette.accent = value.to_string(),
                _ => {}
            }
        }
    }

    Ok(palette)
}

pub fn parse_matugen_json(path: &str) -> Result<Palette, ColorError> {
    let content = std::fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;

    let mut palette = Palette::default();

    if let Some(colors) = json.get("colors") {
        if let Some(primary) = colors.get("primary").and_then(|v| v.as_str()) {
            palette.accent = format!("#{primary}");
        }
        if let Some(background) = colors.get("background").and_then(|v| v.as_str()) {
            palette.background = format!("#{background}");
        }
        if let Some(on_background) = colors.get("on_background").and_then(|v| v.as_str()) {
            palette.foreground = format!("#{on_background}");
        }
    }

    Ok(palette)
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
