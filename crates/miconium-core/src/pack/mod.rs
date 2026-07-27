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
    pub categories: HashMap<String, HashMap<String, Vec<LayerData>>>,
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

        let mut categories = HashMap::new();
        let mut sign_dirs: Vec<_> = std::fs::read_dir(&signs_path)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        sign_dirs.sort();

        for dir in &sign_dirs {
            let cat_name = dir
                .file_name()
                .map_or_else(|| "unknown".into(), |s| s.to_string_lossy().into_owned());

            let mut variants: HashMap<String, Vec<LayerData>> = HashMap::new();
            let mut subdirs: Vec<_> = std::fs::read_dir(dir)
                .map(|rd| {
                    rd.filter_map(Result::ok)
                        .map(|e| e.path())
                        .filter(|p| p.is_dir())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            subdirs.sort();

            if subdirs.is_empty() {
                let layers = read_svg_dir(dir).unwrap_or_default();
                if !layers.is_empty() {
                    variants.insert("scalable".into(), layers);
                }
            } else {
                for subdir in &subdirs {
                    let variant_name = subdir
                        .file_name()
                        .map_or_else(|| "unknown".into(), |s| s.to_string_lossy().into_owned());
                    let layers = read_svg_dir(subdir).unwrap_or_default();
                    if !layers.is_empty() {
                        variants.insert(variant_name, layers);
                    }
                }
            }

            if !variants.is_empty() {
                categories.insert(cat_name, variants);
            }
        }

        let signs = Signs { categories };

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
        self.signs.categories.get(category).map_or_else(Vec::new, |variants| {
            variants.values().flatten().cloned().collect()
        })
    }

    #[must_use]
    pub fn get_category_variants(&self, category: &str) -> Vec<String> {
        self.signs.categories.get(category).map_or_else(Vec::new, |variants| {
            let mut keys: Vec<String> = variants.keys().cloned().collect();
            keys.sort();
            keys
        })
    }

    #[must_use]
    pub fn get_sign_variant(&self, category: &str, variant: &str) -> Vec<LayerData> {
        self.signs.categories.get(category)
            .and_then(|v| v.get(variant))
            .cloned()
            .unwrap_or_default()
    }

    #[must_use]
    pub fn all_signs(&self) -> HashMap<String, Vec<LayerData>> {
        self.signs.categories.iter().map(|(cat, variants)| {
            let all: Vec<LayerData> = variants.values().flatten().cloned().collect();
            (cat.clone(), all)
        }).collect()
    }

    #[must_use]
    pub fn categories(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.signs.categories.keys().cloned().collect();
        keys.sort();
        keys
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
        self.signs.categories.values()
            .flat_map(|variants| variants.values())
            .map(Vec::len)
            .sum()
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
