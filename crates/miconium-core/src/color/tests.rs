use super::*;
use crate::config::ColorOverrides;

const XDG_COLORS: &str = r"
# XDG color scheme
Foreground: #cdd6f4
Background: #1e1e2e
Accent: #89b4fa
";

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
    assert_eq!(palette.background, "#1e1e2e");
    assert_eq!(palette.foreground, "#cdd6f4");
}

const KDE_XDG: &str = r"
[General]
ColorScheme=DankMatugen

[Colors:Window]
BackgroundNormal=20,20,12
ForegroundNormal=230,226,213

[Colors:View]
BackgroundNormal=32,31,24
ForegroundNormal=230,226,213

[Colors:Selection]
BackgroundNormal=100,97,21
";

#[test]
fn parse_xdg_kde_format() {
    let palette = parse_xdg_colors(KDE_XDG).unwrap();
    assert_eq!(palette.background, "#14140c");
    assert_eq!(palette.foreground, "#e6e2d5");
    assert_eq!(palette.accent, "#646115");
    assert_eq!(palette.roles.get("on_surface").map(String::as_str), Some("#e6e2d5"));
}

const DMS_MATUGEN: &str = r##"{
  "dank16": { "color0": {"dark": "#14140c", "light": "#ffffff" } },
  "colors": {
    "dark": {
      "background": "#14140c",
      "on_background": "#e6e2d5",
      "primary": "#cfca74",
      "primary_container": "#4c4900",
      "surface": "#202018",
      "on_surface": "#e6e2d5",
      "secondary": "#64600e",
      "on_secondary": "#343200"
    },
    "light": { "background": "#ffffff", "primary": "#0000ff" }
  }
}"##;

#[test]
fn parse_matugen_dms_nested() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dms-colors.json");
    std::fs::write(&path, DMS_MATUGEN).unwrap();
    let palette = parse_matugen_json(&path.to_string_lossy()).unwrap();
    // Prefers the dark scheme.
    assert_eq!(palette.background, "#14140c");
    assert_eq!(palette.foreground, "#e6e2d5");
    assert_eq!(palette.accent, "#cfca74");
    assert_eq!(palette.roles.get("secondary").map(String::as_str), Some("#64600e"));
    assert_eq!(palette.roles.get("on_surface").map(String::as_str), Some("#e6e2d5"));
}

#[test]
fn layer_map_rebinds_layer_colors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("matugen.json");
    std::fs::write(&path, DMS_MATUGEN).unwrap();
    let palette = parse_matugen_json(&path.to_string_lossy()).unwrap();

    let map = crate::config::LayerMap {
        frame: "surface".into(),
        sign: "on_surface".into(),
        accessory: "primary_container".into(),
    };
    let mapped = palette.with_layer_map(&map);
    assert_eq!(mapped.background, "#202018");
    assert_eq!(mapped.foreground, "#e6e2d5");
    assert_eq!(mapped.accent, "#4c4900");
}

#[test]
fn layer_map_unknown_role_is_noop() {
    let palette = Palette {
        foreground: "#ffffff".into(),
        background: "#111111".into(),
        accent: "#89b4fa".into(),
        roles: std::collections::HashMap::new(),
    };
    let map = crate::config::LayerMap {
        frame: "does_not_exist".into(),
        sign: "foreground".into(),
        accessory: "accent".into(),
    };
    let mapped = palette.with_layer_map(&map);
    assert_eq!(mapped.background, "#111111");
    assert_eq!(mapped.foreground, palette.foreground);
}

#[test]
fn role_names_includes_roles() {
    let mut roles = std::collections::HashMap::new();
    roles.insert("secondary_container".into(), "#222222".into());
    let palette = Palette {
        foreground: "#ffffff".into(),
        background: "#1e1e2e".into(),
        accent: "#89b4fa".into(),
        roles,
    };
    let names = palette.role_names();
    assert!(names.contains(&"secondary_container".to_string()));
    assert!(names.contains(&"foreground".to_string()));
}

#[test]
fn probe_finds_fixed_matugen_candidate() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join(".config/matugen");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("colors.json"), "{}").unwrap();
    let found = probe_in_root(root.path(), ColorSourceKind::Matugen);
    assert_eq!(found, Some(dir.join("colors.json")));
}

#[test]
fn probe_scans_directory_for_json_in_subegg() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join(".config/dank/.dms");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("dms-colors.json"), "{}").unwrap();
    let found = probe_in_root(root.path(), ColorSourceKind::Matugen);
    assert!(found.is_some());
    assert!(found.unwrap().ends_with("dms-colors.json"));
}

#[test]
fn probe_returns_none_when_nothing_matches() {
    let root = tempfile::tempdir().unwrap();
    assert!(probe_in_root(root.path(), ColorSourceKind::Matugen).is_none());
    assert!(probe_in_root(root.path(), ColorSourceKind::Xdg).is_none());
    assert!(probe_in_root(root.path(), ColorSourceKind::Manual).is_none());
}
