use super::*;
use crate::config::ColorOverrides;

const XDG_COLORS: &str = r#"
# XDG color scheme
Foreground: #cdd6f4
Background: #1e1e2e
Accent: #89b4fa
"#;

const MANUAL_TOML: &str = r##"
[manual]
bottom = "#282828"
top = "#FFA700"
"##;

#[test]
fn parse_xdg_colors_basic() {
    let palette = parse_xdg_colors(XDG_COLORS).unwrap();
    assert_eq!(palette.foreground, "#cdd6f4");
    assert_eq!(palette.background, "#1e1e2e");
    assert_eq!(palette.accent, "#89b4fa");
}

#[test]
fn parse_xdg_colors_empty() {
    let palette = parse_xdg_colors("").unwrap();
    assert_eq!(palette, Palette::default());
}

#[test]
fn parse_xdg_colors_commented() {
    let palette = parse_xdg_colors("# only comment").unwrap();
    assert_eq!(palette, Palette::default());
}

#[test]
fn parse_xdg_colors_case_insensitive() {
    let content = "Foreground: #000000\nBackground: #ffffff\n";
    let palette = parse_xdg_colors(content).unwrap();
    assert_eq!(palette.foreground, "#000000");
    assert_eq!(palette.background, "#ffffff");
}

#[test]
fn parse_xdg_colors_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.colors");
    std::fs::write(&path, XDG_COLORS).unwrap();
    let palette = parse_colors_file(&path.to_string_lossy()).unwrap();
    assert_eq!(palette.accent, "#89b4fa");
}

#[test]
fn parse_manual_toml_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("colors.toml");
    std::fs::write(&path, MANUAL_TOML).unwrap();
    let palette = parse_manual_toml(&path.to_string_lossy()).unwrap();
    assert_eq!(palette.background, "#282828");
    assert_eq!(palette.surface, "#282828");
    assert_eq!(palette.accent, "#FFA700");
}

#[test]
fn parse_manual_toml_partial() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("colors.toml");
    std::fs::write(
        &path,
        r##"[manual]
top = "#ff0000"
"##,
    )
    .unwrap();
    let palette = parse_manual_toml(&path.to_string_lossy()).unwrap();
    assert_eq!(palette.accent, "#ff0000");
    assert_eq!(palette.background, Palette::default().background);
}

#[test]
fn color_source_from_config_prioritizes_scheme() {
    let mut config = ColorsConfig::default();
    config.scheme = Some("scheme.colors".into());
    config.matugen = Some("matugen.json".into());
    config.manual = Some("colors.toml".into());

    let source = ColorSource::from_config(&config).unwrap();
    assert!(matches!(source, ColorSource::XdgColors(_)));
}

#[test]
fn color_source_from_config_fallback() {
    let mut config = ColorsConfig::default();
    config.manual = Some("colors.toml".into());

    let source = ColorSource::from_config(&config).unwrap();
    assert!(matches!(source, ColorSource::ManualToml(_)));
}

#[test]
fn color_source_from_config_none() {
    let config = ColorsConfig::default();
    assert!(ColorSource::from_config(&config).is_none());
}

#[test]
fn resolve_palette_applies_overrides() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("colors.toml");
    std::fs::write(&path, MANUAL_TOML).unwrap();

    let mut config = ColorsConfig::default();
    config.manual = Some(path.to_string_lossy().into());
    config.overrides = Some(ColorOverrides {
        foreground: Some("#ff0000".into()),
        background: None,
        accent: None,
        surface: None,
        error: None,
    });

    let palette = resolve_palette(&config).unwrap();
    assert_eq!(palette.foreground, "#ff0000");
    assert_eq!(palette.background, "#282828");
    assert_eq!(palette.accent, "#FFA700");
}

#[test]
fn resolve_palette_no_source() {
    let config = ColorsConfig::default();
    let err = resolve_palette(&config).unwrap_err();
    assert!(matches!(err, ColorError::NoSource));
}

#[test]
fn palette_apply_overrides_none() {
    let palette = Palette::default();
    let overrides = ColorOverrides {
        foreground: None,
        background: None,
        accent: None,
        surface: None,
        error: None,
    };
    let result = palette.apply_overrides(&overrides);
    assert_eq!(result.foreground, Palette::default().foreground);
}

#[test]
fn palette_apply_overrides_partial() {
    let palette = Palette::default();
    let overrides = ColorOverrides {
        foreground: Some("#111111".into()),
        background: None,
        accent: None,
        surface: None,
        error: None,
    };
    let result = palette.apply_overrides(&overrides);
    assert_eq!(result.foreground, "#111111");
    assert_eq!(result.background, Palette::default().background);
}

#[test]
fn parse_matugen_json_basic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("matugen.json");
    std::fs::write(
        &path,
        r#"{
  "colors": {
    "primary": "89b4fa",
    "surface": "313244",
    "background": "1e1e2e",
    "on_background": "cdd6f4",
    "error": "f38ba8"
  }
}"#,
    )
    .unwrap();
    let palette = parse_matugen_json(&path.to_string_lossy()).unwrap();
    assert_eq!(palette.accent, "#89b4fa");
    assert_eq!(palette.surface, "#313244");
    assert_eq!(palette.background, "#1e1e2e");
    assert_eq!(palette.foreground, "#cdd6f4");
    assert_eq!(palette.error, "#f38ba8");
}
