use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackError {
    #[error("pack not found: {0}")]
    NotFound(String),
    #[error("missing required directory: {0}")]
    MissingRequired(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct Pack {
    pub name: String,
    pub path: PathBuf,
    pub frames: Frames,
    pub signs: Signs,
    pub accessories: Vec<LayerData>,
}

#[derive(Debug, Clone)]
pub struct Frames {
    pub colorizable: Vec<LayerData>,
    pub static_frames: Option<StaticFrames>,
}

#[derive(Debug, Clone)]
pub struct StaticFrames {
    pub dark: Vec<LayerData>,
    pub light: Vec<LayerData>,
}

#[derive(Debug, Clone)]
pub struct Signs {
    pub actions: Vec<LayerData>,
    pub apps: Vec<LayerData>,
    pub categories: Vec<LayerData>,
    pub devices: Vec<LayerData>,
    pub emblems: Vec<LayerData>,
    pub mime: Vec<LayerData>,
    pub places: Vec<LayerData>,
    pub preferences: Vec<LayerData>,
    pub status: Vec<LayerData>,
}

#[derive(Debug, Clone)]
pub struct LayerData {
    pub svg_content: String,
    pub name: String,
}

fn stem(filename: &str) -> String {
    Path::new(filename)
        .file_stem()
        .map_or_else(|| filename.to_string(), |s| s.to_string_lossy().into_owned())
}

impl Pack {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, PackError> {
        let root = path.as_ref();
        if !root.is_dir() {
            return Err(PackError::NotFound(root.display().to_string()));
        }

        let name = root
            .file_name()
            .map_or_else(|| "unknown".into(), |s| s.to_string_lossy().into_owned());

        let frames_path = root.join("frames");
        if !frames_path.is_dir() {
            return Err(PackError::MissingRequired("frames/".into()));
        }

        let frames = Frames {
            colorizable: read_svg_dir(&frames_path.join("colorizable"))?,
            static_frames: read_static_frames(&frames_path.join("static")),
        };

        let signs_path = root.join("signs");
        if !signs_path.is_dir() {
            return Err(PackError::MissingRequired("signs/".into()));
        }

        let signs = Signs {
            actions: read_svg_dir(&signs_path.join("actions")).unwrap_or_default(),
            apps: read_svg_dir(&signs_path.join("apps"))?,
            categories: read_svg_dir(&signs_path.join("categories"))?,
            devices: read_svg_dir(&signs_path.join("devices"))?,
            emblems: read_svg_dir(&signs_path.join("emblems"))?,
            mime: read_svg_dir(&signs_path.join("mime"))?,
            places: read_svg_dir(&signs_path.join("places")).unwrap_or_default(),
            preferences: read_svg_dir(&signs_path.join("preferences"))?,
            status: read_svg_dir(&signs_path.join("status"))?,
        };

        let accessories = read_svg_dir(&root.join("accessories")).unwrap_or_default();

        Ok(Self {
            name,
            path: root.to_path_buf(),
            frames,
            signs,
            accessories,
        })
    }

    #[must_use]
    pub fn get_signs_by_category(&self, category: &str) -> Vec<LayerData> {
        self.signs_by_category_ref(category).to_vec()
    }

    #[must_use]
    pub fn signs_by_category_ref(&self, category: &str) -> &[LayerData] {
        match category {
            "actions" => &self.signs.actions,
            "apps" => &self.signs.apps,
            "categories" => &self.signs.categories,
            "devices" => &self.signs.devices,
            "emblems" => &self.signs.emblems,
            "mime" => &self.signs.mime,
            "places" => &self.signs.places,
            "preferences" => &self.signs.preferences,
            "status" => &self.signs.status,
            _ => &[],
        }
    }

    #[must_use]
    pub fn all_signs(&self) -> HashMap<String, Vec<LayerData>> {
        let mut map = HashMap::new();
        for category in &[
            "actions", "apps", "categories", "devices", "emblems", "mime", "places",
            "preferences", "status",
        ] {
            map.insert(category.to_string(), self.get_signs_by_category(category));
        }
        map
    }

    #[must_use]
    pub fn has_colorizable_frames(&self) -> bool {
        !self.frames.colorizable.is_empty()
    }

    #[must_use]
    pub fn has_static_frames(&self) -> bool {
        self.frames.static_frames.is_some()
    }

    #[must_use]
    pub fn sign_count(&self) -> usize {
        self.signs.actions.len()
            + self.signs.apps.len()
            + self.signs.categories.len()
            + self.signs.devices.len()
            + self.signs.emblems.len()
            + self.signs.mime.len()
            + self.signs.places.len()
            + self.signs.preferences.len()
            + self.signs.status.len()
    }
}

fn read_svg_dir(dir: &Path) -> Result<Vec<LayerData>, PackError> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "svg"))
        .collect();

    entries.sort();

    let mut layers = Vec::with_capacity(entries.len());
    for path in &entries {
        let svg_content = std::fs::read_to_string(path)?;
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .map(|f| stem(&f))
            .unwrap_or_default();
        layers.push(LayerData { svg_content, name });
    }

    Ok(layers)
}

fn read_static_frames(dir: &Path) -> Option<StaticFrames> {
    if !dir.is_dir() {
        return None;
    }

    let dark = read_svg_dir(&dir.join("Dark")).unwrap_or_default();
    let light = read_svg_dir(&dir.join("Light")).unwrap_or_default();

    if dark.is_empty() && light.is_empty() {
        return None;
    }

    Some(StaticFrames { dark, light })
}

#[cfg(test)]
mod tests;
