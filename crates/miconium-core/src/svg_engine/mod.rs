#![allow(clippy::format_push_string)]

use crate::color::Palette;
use crate::pack::LayerData;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SvgError {
    #[error("failed to parse SVG: {0}")]
    Parse(String),
    #[error("failed to assemble icon: {0}")]
    Assembly(String),
    #[error("no layers provided")]
    NoLayers,
}

#[derive(Debug, Clone)]
pub struct AssembledIcon {
    pub svg: String,
    pub name: String,
}

/// Extract the body content (between `<svg>` and `</svg>`) from an SVG string.
fn extract_svg_body(svg: &str) -> Option<&str> {
    let start = svg.find("<svg")?;
    let after_open = svg[start..].find('>')?;
    let body_start = start + after_open + 1;

    let close_tag = svg[body_start..].rfind("</svg>")?;
    let body_end = body_start + close_tag;

    if body_end <= body_start {
        return Some("");
    }
    Some(&svg[body_start..body_end])
}

/// Extract viewBox from an SVG string using roxmltree.
fn extract_viewbox(svg: &str) -> Result<String, SvgError> {
    let doc = roxmltree::Document::parse(svg).map_err(|e| SvgError::Parse(e.to_string()))?;
    let root = doc.root_element();
    Ok(root
        .attribute("viewBox")
        .unwrap_or("0 0 24 24")
        .to_string())
}

/// Extract the width/height attributes for use in the output SVG.
fn extract_dimensions(svg: &str) -> (Option<String>, Option<String>) {
    let doc = roxmltree::Document::parse(svg);
    match doc {
        Ok(doc) => {
            let root = doc.root_element();
            let width = root.attribute("width").map(String::from);
            let height = root.attribute("height").map(String::from);
            (width, height)
        }
        Err(_) => (None, None),
    }
}

fn build_svg_tag(
    viewbox: &str,
    width: Option<&str>,
    height: Option<&str>,
    body: &str,
) -> String {
    let mut attrs = format!(r#"xmlns="http://www.w3.org/2000/svg" viewBox="{viewbox}""#);
    if let Some(w) = width {
        attrs.push_str(&format!(r#" width="{w}""#));
    }
    if let Some(h) = height {
        attrs.push_str(&format!(r#" height="{h}""#));
    }
    format!("<svg {attrs}>{body}</svg>")
}

/// Merge multiple SVG layers into one, wrapping each in a `<g>` group.
/// The first layer provides the viewBox and dimensions for the output.
pub fn merge_layers(layers: &[LayerData]) -> Result<String, SvgError> {
    let first = layers.first().ok_or(SvgError::NoLayers)?;
    let viewbox = extract_viewbox(&first.svg_content)?;
    let (width, height) = extract_dimensions(&first.svg_content);

    let mut body = String::new();

    for (i, layer) in layers.iter().enumerate() {
        let inner = extract_svg_body(&layer.svg_content).unwrap_or("");
        if inner.is_empty() {
            continue;
        }
        body.push_str(&format!(r#"<g id="layer-{i}">"#));
        body.push_str(inner);
        body.push_str("</g>");
    }

    Ok(build_svg_tag(&viewbox, width.as_deref(), height.as_deref(), &body))
}

/// Build final icon from frame + sign + optional accessories.
pub fn assemble_icon(
    frame: &LayerData,
    sign: &LayerData,
    accessories: &[LayerData],
    palette: &Palette,
) -> Result<AssembledIcon, SvgError> {
    let mut layers = Vec::with_capacity(2 + accessories.len());
    layers.push(frame.clone());
    layers.push(sign.clone());
    layers.extend_from_slice(accessories);

    let merged = merge_layers(&layers)?;
    let colored = inject_colors(&merged, palette);

    Ok(AssembledIcon {
        svg: colored,
        name: format!("{}_{}", frame.name, sign.name),
    })
}

/// Inject colors into a single SVG (for previews).
pub fn assemble_single(svg: &str, palette: &Palette) -> Result<AssembledIcon, SvgError> {
    let colored = inject_colors(svg, palette);
    Ok(AssembledIcon {
        svg: colored,
        name: "preview".into(),
    })
}

/// Replace color placeholder tokens with actual hex values from the palette.
fn inject_colors(svg: &str, palette: &Palette) -> String {
    let mut result = svg.to_string();

    let replacements = [
        ("currentColor", palette.foreground.as_str()),
        ("currentForeground", palette.foreground.as_str()),
        ("currentBackground", palette.background.as_str()),
        ("currentAccent", palette.accent.as_str()),
        ("currentSurface", palette.surface.as_str()),
        ("currentError", palette.error.as_str()),
    ];

    for (from, to) in &replacements {
        result = result.replace(from, to);
    }

    result
}

#[cfg(test)]
mod tests;
