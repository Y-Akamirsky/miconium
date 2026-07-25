use super::*;

const FRAME_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48">
  <rect x="2" y="2" width="44" height="44" rx="8" fill="currentBackground"/>
</svg>"#;

const SIGN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <circle cx="12" cy="12" r="10" fill="currentForeground"/>
</svg>"#;

const ACCESSORY_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48">
  <rect x="4" y="4" width="40" height="40" rx="10" fill="currentAccent" opacity="0.3"/>
</svg>"#;

fn make_layer(svg: &str, name: &str) -> LayerData {
    LayerData {
        svg_content: svg.to_string(),
        name: name.to_string(),
    }
}

#[test]
fn extract_body_basic() {
    let body = extract_svg_body(FRAME_SVG).unwrap();
    assert!(body.contains("rect"));
    assert!(!body.contains("<svg"));
    assert!(!body.contains("</svg>"));
}

#[test]
fn extract_body_empty_svg() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"></svg>"#;
    let body = extract_svg_body(svg).unwrap();
    assert!(body.trim().is_empty());
}

#[test]
fn extract_body_no_svg_tag() {
    let result = extract_svg_body("not svg");
    assert!(result.is_none());
}

#[test]
fn extract_viewbox_from_svg() {
    let vb = extract_viewbox(FRAME_SVG).unwrap();
    assert_eq!(vb, "0 0 48 48");
}

#[test]
fn extract_viewbox_missing() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>"#;
    let vb = extract_viewbox(svg).unwrap();
    assert_eq!(vb, "0 0 24 24");
}

#[test]
fn extract_viewbox_invalid_svg() {
    let err = extract_viewbox("not svg").unwrap_err();
    assert!(matches!(err, SvgError::Parse(_)));
}

#[test]
fn merge_two_layers() {
    let layers = vec![make_layer(FRAME_SVG, "frame"), make_layer(SIGN_SVG, "sign")];
    let merged = merge_layers(&layers, &[]).unwrap();
    assert!(merged.starts_with("<svg"));
    assert!(merged.ends_with("</svg>"));
    assert!(merged.contains(r#"viewBox="0 0 48 48""#));
    assert!(merged.contains(r#"id="layer-0""#));
    assert!(merged.contains(r#"id="layer-1""#));
    assert!(merged.contains("<rect"));
    assert!(merged.contains("<circle"));
}

#[test]
fn merge_single_layer() {
    let layers = vec![make_layer(FRAME_SVG, "frame")];
    let merged = merge_layers(&layers, &[]).unwrap();
    assert!(merged.contains("rect"));
    assert!(merged.contains(r#"id="layer-0""#));
}

#[test]
fn merge_no_layers() {
    let layers: Vec<LayerData> = vec![];
    let err = merge_layers(&layers, &[]).unwrap_err();
    assert!(matches!(err, SvgError::NoLayers));
}

#[test]
fn merge_with_empty_layer() {
    let empty = make_layer(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"></svg>"#,
        "empty",
    );
    let layers = vec![make_layer(FRAME_SVG, "frame"), empty];
    let merged = merge_layers(&layers, &[]).unwrap();
    assert!(merged.contains(r#"id="layer-0""#));
}

#[test]
fn inject_colors_replaces_placeholders() {
    let palette = Palette {
        foreground: "#ffffff".into(),
        background: "#000000".into(),
        accent: "#ff0000".into(),
    };
    let svg = concat!(
        r#"<rect fill="currentForeground" stroke="currentBackground"/> "#,
        r#"<circle fill="currentAccent"/> "#,
        r#"<path fill="currentColor"/>"#,
    );
    let result = inject_colors(svg, &palette, false);
    assert!(result.contains(r##"fill="#ffffff""##));
    assert!(result.contains(r##"stroke="#000000""##));
    assert!(result.contains(r##"fill="#ff0000""##));
    assert!(result.contains(r##"fill="#ffffff""##));
}

#[test]
fn inject_colors_preserves_unknown_tokens() {
    let palette = Palette::default();
    let svg = r#"<rect fill="currentColor" unknown="someToken"/>"#;
    let result = inject_colors(svg, &palette, false);
    assert!(result.contains(r##"fill="#ffffff""##));
    assert!(result.contains(r#"unknown="someToken""#));
}

#[test]
fn assemble_icon_combines_all_layers() {
    let frame = make_layer(FRAME_SVG, "frame_default");
    let sign = make_layer(SIGN_SVG, "apps_firefox");
    let acc = make_layer(ACCESSORY_SVG, "shadow");
    let palette = Palette::default();

    let icon = assemble_icon(&frame, &sign, &[acc], &palette, true, true, false, 1.0, 1.0, 1.0).unwrap();
    assert_eq!(icon.name, "frame_default_apps_firefox");
    assert!(icon.svg.contains("rect"));
    assert!(icon.svg.contains("circle"));
    assert!(icon.svg.contains(r#"id="layer-0""#));
    assert!(icon.svg.contains(r#"id="layer-1""#));
    assert!(icon.svg.contains(r#"id="layer-2""#));
    assert!(icon.svg.contains(r#"viewBox="0 0 48 48""#));
}

#[test]
fn assemble_icon_no_accessories() {
    let frame = make_layer(FRAME_SVG, "frame");
    let sign = make_layer(SIGN_SVG, "sign");
    let palette = Palette::default();

    let icon = assemble_icon(&frame, &sign, &[], &palette, true, true, false, 1.0, 1.0, 1.0).unwrap();
    assert!(icon.svg.contains("rect"));
    assert!(icon.svg.contains("circle"));
    // 3 groups: layer-0 (frame), fill-wrapper, layer-1 (sign)
    assert_eq!(icon.svg.matches("</g>").count(), 3);
}

#[test]
fn assemble_single_injects_colors() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
      <rect fill="currentColor"/>
    </svg>"#;
    let palette = Palette {
        foreground: "#abc123".into(),
        ..Palette::default()
    };

    let result = assemble_single(svg, &palette).unwrap();
    assert_eq!(result.name, "preview");
    assert!(result.svg.contains(r##"fill="#abc123""##));
}

#[test]
fn merge_layers_preserves_svg_structure() {
    let layers = vec![make_layer(FRAME_SVG, "frame"), make_layer(SIGN_SVG, "sign")];
    let merged = merge_layers(&layers, &[]).unwrap();
    let doc = roxmltree::Document::parse(&merged).unwrap();
    assert_eq!(doc.root_element().tag_name().name(), "svg");
    let children: Vec<_> = doc
        .root_element()
        .children()
        .filter(|n| n.is_element())
        .collect();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].tag_name().name(), "g");
    assert_eq!(children[1].tag_name().name(), "g");
}

#[test]
fn inject_colors_multiple_occurrences() {
    let palette = Palette {
        foreground: "#ff0000".into(),
        ..Palette::default()
    };
    let svg = r#"<path fill="currentColor"/><circle fill="currentColor"/>"#;
    let result = inject_colors(svg, &palette, false);
    assert_eq!(result.matches(r##"fill="#ff0000""##).count(), 2);
}
