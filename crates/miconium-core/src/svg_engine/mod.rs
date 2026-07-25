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

/// Extract a viewBox from an SVG. If the SVG has no explicit viewBox,
/// construct one from width/height attributes (e.g. `0 0 64 64`).
/// Falls back to `0 0 24 24` if neither viewBox nor width/height exist.
fn extract_viewbox(svg: &str) -> Result<String, SvgError> {
    let doc = roxmltree::Document::parse(svg).map_err(|e| SvgError::Parse(e.to_string()))?;
    let root = doc.root_element();
    if let Some(vb) = root.attribute("viewBox") {
        return Ok(vb.to_string());
    }
    let w = root.attribute("width").and_then(|s| s.parse::<f64>().ok());
    let h = root.attribute("height").and_then(|s| s.parse::<f64>().ok());
    if let (Some(w), Some(h)) = (w, h) {
        return Ok(format!("0 0 {w} {h}"));
    }
    Ok("0 0 24 24".into())
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

/// Parse viewBox from SVG attributes, falling back to width/height, then `0 0 24 24`.
fn parse_viewbox_values(svg: &str) -> Option<(f64, f64, f64, f64)> {
    let doc = roxmltree::Document::parse(svg).ok()?;
    let root = doc.root_element();
    let vb = root.attribute("viewBox").map(String::from).or_else(|| {
        let w = root.attribute("width")?.parse::<f64>().ok()?;
        let h = root.attribute("height")?.parse::<f64>().ok()?;
        Some(format!("0 0 {w} {h}"))
    })?;
    let parts: Vec<f64> = vb.split_whitespace().filter_map(|s| s.parse().ok()).collect();
    if parts.len() == 4 {
        Some((parts[0], parts[1], parts[2], parts[3]))
    } else {
        None
    }
}

fn build_svg_tag(
    viewbox: &str,
    width: Option<&str>,
    height: Option<&str>,
    body: &str,
    extra_namespaces: &str,
) -> String {
    let mut attrs = format!(r#"xmlns="http://www.w3.org/2000/svg" viewBox="{viewbox}""#);
    if let Some(w) = width {
        attrs.push_str(&format!(r#" width="{w}""#));
    }
    if let Some(h) = height {
        attrs.push_str(&format!(r#" height="{h}""#));
    }
    if !extra_namespaces.is_empty() {
        attrs.push(' ');
        attrs.push_str(extra_namespaces);
    }
    format!("<svg {attrs}>{body}</svg>")
}

/// Collect unique `xmlns:*` declarations from an SVG.
fn collect_namespaces(svg: &str) -> Vec<String> {
    let mut decls: Vec<String> = Vec::new();
    if let Some(start) = svg.find("<svg") {
        if let Some(end) = svg[start..].find('>') {
            let open_tag = &svg[start..=start + end];
            for attr in open_tag.split_whitespace() {
                let attr = attr.trim_end_matches('/').trim_end_matches('>');
                if attr.starts_with("xmlns") {
                    let prefix = attr.split('=').next().unwrap_or(attr);
                    if !decls.iter().any(|d| d.starts_with(prefix)) {
                        decls.push(attr.to_string());
                    }
                }
            }
        }
    }
    decls
}

/// Merge multiple SVG layers into one, wrapping each in a `<g>` group.
/// The first layer provides the viewBox and dimensions for the output.
/// Subsequent layers are centered within the first layer's viewBox.
/// `user_scales` — optional per-layer multipliers applied on top of auto-fit scaling.
pub fn merge_layers(layers: &[LayerData], user_scales: &[f64]) -> Result<String, SvgError> {
    let first = layers.first().ok_or(SvgError::NoLayers)?;
    let viewbox = extract_viewbox(&first.svg_content)?;
    let (width, height) = extract_dimensions(&first.svg_content);

    let ref_vb = parse_viewbox_values(&first.svg_content);

    // Collect xmlns:* declarations from all layers (deduplicated)
    let mut ns_decls: Vec<String> = Vec::new();
    for layer in layers {
        for decl in collect_namespaces(&layer.svg_content) {
            let prefix = decl.split('=').next().unwrap_or(&decl).to_string();
            if !ns_decls.iter().any(|d| d.starts_with(&prefix)) {
                ns_decls.push(decl);
            }
        }
    }
    let extra_ns = ns_decls.join(" ");

    let mut body = String::new();

    for (i, layer) in layers.iter().enumerate() {
        let inner = extract_svg_body(&layer.svg_content).unwrap_or("");
        if inner.is_empty() {
            continue;
        }

        let mut tag = format!(r#"<g id="layer-{i}""#);

        if i > 0 {
            if let Some((rx, ry, rw, rh)) = ref_vb {
                if let Some((lx, ly, lw, lh)) = parse_viewbox_values(&layer.svg_content) {
                    if lw > 0.0 && lh > 0.0 {
                        let user_s = user_scales.get(i).copied().unwrap_or(1.0);
                        let s = (rw / lw).min(rh / lh) * user_s;
                        let dx = rx + (rw - lw * s) / 2.0 - lx * s;
                        let dy = ry + (rh - lh * s) / 2.0 - ly * s;
                        tag.push_str(&format!(
                            r#" transform="translate({dx:.3} {dy:.3}) scale({s:.3})""#,
                        ));
                    }
                }
            }
        }

        tag.push('>');
        body.push_str(&tag);
        body.push_str(inner);
        body.push_str("</g>");
    }

    Ok(build_svg_tag(&viewbox, width.as_deref(), height.as_deref(), &body, &extra_ns))
}

#[allow(clippy::too_many_arguments)]
/// Build final icon from frame + sign + optional accessories.
/// When `use_frame` is false, the sign is rendered standalone
/// and `currentColor` maps to the background color (for no-frame icons).
/// When `frame_is_static` is true, the frame layer is NOT colorized.
/// Accessories whose name starts with `"st-"` are NOT colorized.
/// `frame_scale`, `icon_scale`, `acc_scale` multiply the auto-calculated sizes.
pub fn assemble_icon(
    frame: &LayerData,
    sign: &LayerData,
    accessories: &[LayerData],
    palette: &Palette,
    use_frame: bool,
    use_accessories: bool,
    frame_is_static: bool,
    frame_scale: f64,
    icon_scale: f64,
    acc_scale: f64,
) -> Result<AssembledIcon, SvgError> {
    let mut layers = Vec::with_capacity(2 + accessories.len());
    let mut layer_scales: Vec<f64> = Vec::new();

    if use_frame {
        let mut f = frame.clone();
        if !frame_is_static {
            f.svg_content = inject_colors(&f.svg_content, palette, false);
            f.svg_content = replace_hardcoded_colors(&f.svg_content, &palette.background);
        }
        layers.push(f);
        layer_scales.push(frame_scale);
    }

    let mut s = sign.clone();
    s.svg_content = inject_colors(&s.svg_content, palette, !use_frame);
    s.svg_content = replace_hardcoded_colors(&s.svg_content, &palette.foreground);
    layers.push(s);
    layer_scales.push(icon_scale);

    if use_accessories {
        for acc in accessories {
            let mut a = acc.clone();
            if !a.name.starts_with("st-") {
                a.svg_content = inject_colors(&a.svg_content, palette, false);
                a.svg_content = replace_hardcoded_colors(&a.svg_content, &palette.accent);
            }
            layers.push(a);
            layer_scales.push(acc_scale);
        }
    }

    let merged = merge_layers(&layers, &layer_scales)?;

    let name = if use_frame {
        format!("{}_{}", frame.name, sign.name)
    } else {
        sign.name.clone()
    };
    Ok(AssembledIcon { svg: merged, name })
}

/// Inject colors into a single SVG (for previews).
pub fn assemble_single(svg: &str, palette: &Palette) -> Result<AssembledIcon, SvgError> {
    let colored = inject_colors(svg, palette, false);
    Ok(AssembledIcon {
        svg: colored,
        name: "preview".into(),
    })
}

/// Replace hardcoded hex fill/stroke colors in an SVG with `target`.
/// Handles `fill="#XXXXXX"`, `stroke="#XXXXXX"`, `stop-color="#XXXXXX"`,
/// and `fill:#XXXXXX`/`stroke:#XXXXXX`/`stop-color:#XXXXXX` inside `style=""`.
/// Skips `url(#…)`.
#[must_use]
pub fn replace_hardcoded_colors(svg: &str, target: &str) -> String {
    let mut result = String::with_capacity(svg.len());
    let bytes = svg.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'#' {
            let before = &svg[..i];
            // Skip url(#...)
            if let Some(rp) = before.rfind("url(") {
                let after_rp = &before[rp + 4..].trim_start();
                if after_rp.is_empty() || after_rp.as_bytes()[0] == b'#' {
                    if let Some(end) = svg[i..].find(')') {
                        result.push_str(&svg[i..=i + end]);
                        i += end + 1;
                        continue;
                    }
                }
            }

            // Determine context: is # preceded by color keyword like fill/stroke/stop-color?
            let mut context_ok = false;
            for kw in &["fill", "stroke", "stop-color"] {
                // Search backward from end of `before` for the keyword
                if let Some(pos) = before.rfind(kw) {
                    let trailing = &before[pos + kw.len()..];
                    // After keyword we expect `="`, `:`, or `: `, possibly with spaces
                    let trimmed = trailing.trim();
                    if trimmed == "=" || trimmed.starts_with("=\"") || trimmed == ":" || trimmed.starts_with(": ") || trimmed.starts_with(":\"") {
                        // Ensure there's no other '=' between kw and # (avoid fill="url(...)
                        context_ok = true;
                        break;
                    }
                }
            }

            if context_ok {
                // Try 6-digit hex (#XXXXXX)
                let rest = &svg[i..];
                if rest.len() > 6 && rest[1..7].chars().all(|c| c.is_ascii_hexdigit()) {
                    result.push_str(target);
                    i += 7;
                    continue;
                }
                // Try 3-digit hex (#XXX)
                if rest.len() > 3 && rest[1..4].chars().all(|c| c.is_ascii_hexdigit()) {
                    result.push_str(target);
                    i += 4;
                    continue;
                }
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

/// Replace color placeholder tokens with actual hex values from the palette.
#[must_use]
pub fn inject_colors(svg: &str, palette: &Palette, current_color_is_background: bool) -> String {
    let mut result = svg.to_string();

    let fg = if current_color_is_background {
        &palette.background
    } else {
        &palette.foreground
    };

    let replacements = [
        ("currentColor", fg.as_str()),
        ("currentForeground", palette.foreground.as_str()),
        ("currentBackground", palette.background.as_str()),
        ("currentAccent", palette.accent.as_str()),
    ];

    for (from, to) in &replacements {
        result = result.replace(from, to);
    }

    result
}

#[cfg(test)]
mod tests;
