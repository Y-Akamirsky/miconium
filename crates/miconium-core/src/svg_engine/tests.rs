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
fn merge_with_xlink_href_gradient() {
    // Simulate out-rnd.svg: accessory with gradient that uses xlink:href
    let frame = make_layer(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48">
            <linearGradient id="shine"><stop offset="0" stop-color="#fff"/></linearGradient>
            <rect fill="url(#shine)" width="48" height="48"/>
        </svg>"##,
        "frame",
    );
    let sign = make_layer(SIGN_SVG, "sign");
    let acc = make_layer(
        r##"<svg xmlns="http://www.w3.org/2000/svg"
                xmlns:xlink="http://www.w3.org/1999/xlink"
                viewBox="0 0 48 48">
            <defs>
                <linearGradient id="g1">
                    <stop offset="0" stop-color="#fff"/>
                </linearGradient>
                <linearGradient id="g2" xlink:href="#g1" x1="0" y1="0" x2="48" y2="48"/>
            </defs>
            <circle cx="24" cy="24" r="20" fill="url(#g2)"/>
        </svg>"##,
        "out-rnd",
    );
    let palette = Palette::default();
    let icon = assemble_icon(
        &frame, &sign, &[acc], &palette,
        true, true, false, 1.0, 1.0, 1.0,
    ).unwrap();
    // Must be valid XML
    let doc = roxmltree::Document::parse(&icon.svg).unwrap();
    assert_eq!(doc.root_element().tag_name().name(), "svg");
    // Check that gradients from accessory were prefixed
    let svg = &icon.svg;
    assert!(svg.contains(r##"id="l2-g2""##), "accessory gradient id not prefixed: {svg}");
    assert!(svg.contains(r##"url(#l2-g2)"##), "accessory url reference not prefixed: {svg}");
    assert!(svg.contains(r##"xlink:href="#l2-g1""##), "xlink:href ref not prefixed: {svg}");
    // Frame gradient should NOT be prefixed
    assert!(svg.contains(r##"id="shine""##), "frame gradient id should stay unchanged: {svg}");
}

#[test]
fn assemble_icon_with_out_rnd_accessory() {
    // Real out-rnd.svg content: circle with fill:none;stroke:url(#linearGradient10)
    // where linearGradient10 uses xlink:href="#linearGradient9"
    let frame_svg = r##"<svg xmlns="http://www.w3.org/2000/svg"
            xmlns:xlink="http://www.w3.org/1999/xlink"
            xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape"
            xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd"
            viewBox="0 0 48 48" width="64" height="64">
        <defs>
            <linearGradient id="a-0" gradientUnits="userSpaceOnUse">
                <stop offset="0" stop-color="#474747" style="stop-color:#2a2a2a;stop-opacity:1;"/>
                <stop offset="1" stop-color="#333" style="stop-color:#000000;stop-opacity:1;"/>
            </linearGradient>
            <linearGradient id="shine-grad-final-v3" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" stop-color="#ffffff" stop-opacity="0.5"/>
                <stop offset="50%" stop-color="#ffffff" stop-opacity="0"/>
                <stop offset="100%" stop-color="#ffffff" stop-opacity="0.5"/>
            </linearGradient>
        </defs>
        <circle cx="24" cy="24" r="22" fill="url(#a-0)"/>
    </svg>"##;
    let out_rnd = r##"<svg xmlns="http://www.w3.org/2000/svg"
            xmlns:xlink="http://www.w3.org/1999/xlink"
            xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape"
            xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd"
            viewBox="0 0 16.933 16.933" width="64" height="64">
        <sodipodi:namedview id="namedview6" inkscape:current-layer="svg6"/>
        <defs>
            <linearGradient id="linearGradient9" inkscape:collect="always">
                <stop style="stop-color:#ffffff;stop-opacity:1;" offset="0"/>
                <stop style="stop-color:#ffffff;stop-opacity:0;" offset="0.49"/>
                <stop style="stop-color:#ffffff;stop-opacity:1;" offset="0.98"/>
            </linearGradient>
            <linearGradient id="a-0" gradientUnits="userSpaceOnUse">
                <stop offset="0" stop-color="#474747" style="stop-color:#2a2a2a;stop-opacity:1;"/>
                <stop offset="1" stop-color="#333" style="stop-color:#000000;stop-opacity:1;"/>
            </linearGradient>
            <linearGradient inkscape:collect="always"
                xlink:href="#linearGradient9" id="linearGradient10"
                x1="3.22" y1="3.22" x2="13.70" y2="13.70" gradientUnits="userSpaceOnUse"/>
        </defs>
        <defs>
            <linearGradient id="shine-grad-final-v3" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" stop-color="#ffffff" stop-opacity="0.5"/>
                <stop offset="50%" stop-color="#ffffff" stop-opacity="0"/>
                <stop offset="100%" stop-color="#ffffff" stop-opacity="0.5"/>
            </linearGradient>
        </defs>
        <circle style="opacity:0.355491;fill:none;stroke:url(#linearGradient10);stroke-width:0.462342;stroke-linecap:round;stroke-linejoin:round"
            cx="8.4665" cy="8.4665" r="7.177"/>
    </svg>"##;
    let sign_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="10" fill="currentForeground"/>
    </svg>"##;

    let frame = make_layer(frame_svg, "frame-rnd");
    let sign = make_layer(sign_svg, "sign");
    let acc = make_layer(out_rnd, "out-rnd");
    let palette = Palette::default();

    let icon = assemble_icon(
        &frame, &sign, &[acc], &palette,
        true, true, false, 1.0, 1.0, 1.0,
    ).unwrap();

    // Must be valid XML
    let doc = roxmltree::Document::parse(&icon.svg)
        .unwrap_or_else(|e| panic!("invalid SVG: {e}\n---\n{}", icon.svg));

    // Find all circle elements (frame + sign + out-rnd = 3)
    let circles: Vec<_> = doc.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "circle")
        .collect();
    assert_eq!(circles.len(), 3, "expected 3 circles (frame+sign+out-rnd), got {};\n{}", circles.len(), icon.svg);

    // The last circle is the out-rnd one (layer-2)
    let out_rnd_circle = &circles[2];
    let style = out_rnd_circle.attribute("style").unwrap_or("");
    assert!(style.contains("fill:none"), "out-rnd circle should preserve fill:none: {style}");
    assert!(style.contains("stroke:url(#l2-linearGradient10)"),
        "out-rnd circle stroke URL should be prefixed: {style}");
    assert!(style.contains("stroke-width:0.462342"),
        "out-rnd circle should retain stroke-width");

    // Gradient chain should be intact
    let svg_text = &icon.svg;
    assert!(svg_text.contains("id=\"l2-linearGradient10\""),
        "linearGradient10 id not prefixed");
    assert!(svg_text.contains("xlink:href=\"#l2-linearGradient9\""),
        "xlink:href in gradient10 not prefixed: check for double or missing prefix");
    assert!(svg_text.contains("id=\"l2-linearGradient9\""),
        "linearGradient9 id not prefixed");
    // The stroke URL in style must point to prefixed id
    assert!(svg_text.contains("stroke:url(#l2-linearGradient10)"),
        "stroke URL should reference prefixed gradient: {svg_text}");
    // Frame gradient must NOT be prefixed
    assert!(svg_text.contains("id=\"a-0\""),
        "frame gradient a-0 should stay unchanged");
}

#[test]
fn replace_colors_does_not_skip_hash_after_url() {
    // Regression test: auryo.svg has BOTH fill="url(#c)" (earlier)
    // AND style="fill:#000000" (later). The #000000 must STILL be replaced.
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
        <defs>
            <linearGradient id="c">
                <stop offset="0" stop-color="#5490ea"/>
            </linearGradient>
        </defs>
        <path fill="url(#c)" d="M24 11" style="paint-order:normal;fill:#000000"/>
    </svg>"##;
    let result = replace_hardcoded_colors(svg, "#ff0000");
    assert!(result.contains(r##"fill:#ff0000"##),
        "style fill:#000000 should be replaced with target, but got: {result}");
    assert!(result.contains(r##"fill="url(#c)""##),
        "fill=\"url(#c)\" must be preserved: {result}");
}

#[test]
fn assemble_icon_with_auryo_sign_gets_fg_color() {
    // auryo.svg has fill="url(#c)" and style="fill:#000000".
    // The sign must get foreground color applied to its path fill.
    let frame = make_layer(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48">
            <rect width="48" height="48" fill="currentBackground"/>
        </svg>"##,
        "frame",
    );
    // auryo-like sign: url(#c) references, then style fill override
    let auryo = make_layer(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
            <defs>
                <linearGradient id="c">
                    <stop offset="0" stop-color="#5490ea"/>
                </linearGradient>
            </defs>
            <path fill="url(#c)" d="M24 11" style="paint-order:normal;fill:#000000"/>
        </svg>"##,
        "auryo",
    );
    let palette = Palette::default(); // fg = "#ffffff"

    let icon = assemble_icon(
        &frame, &auryo, &[], &palette,
        true, true, false, 1.0, 1.0, 1.0,
    ).unwrap();

    let svg = &icon.svg;
    // The path's style fill must have been replaced with foreground color (#ffffff).
    assert!(svg.contains(r##"fill:#ffffff"##),
        "style fill in auryo sign should be foreground (#ffffff), got: {svg}");
    // The url reference must remain intact (prefixed as sign is layer-1)
    assert!(svg.contains(r##"fill="url(#l1-c)""##),
        "fill=\"url(#l1-c)\" should be preserved: {svg}");
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
