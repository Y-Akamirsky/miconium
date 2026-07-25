use super::*;

const MINIMAL_TOML: &str = r#"
[pack]
path = "packs/mono"
"#;

const FULL_TOML: &str = r##"
[pack]
path = "packs/mono"
name = "my-custom"

[colors]
scheme = "~/.themes/my.colors"
matugen = "/tmp/matugen.json"

[colors.overrides]
foreground = "#ffffff"
accent = "#ff0000"

[export]
output = "~/.local/share/icons/Miconium"
sizes = [16, 32, 64]
generate_16_symlinks = false

[gui]
window_width = 1024
window_height = 768
"##;

#[test]
fn parse_minimal_config() {
    let config: Config = toml::from_str(MINIMAL_TOML).unwrap();
    assert_eq!(config.pack.path.as_deref(), Some("packs/mono"));
    assert_eq!(config.pack.name, "mono");
    assert!(config.colors.scheme.is_none());
    assert!(config.export.generate_16_symlinks);
}

#[test]
fn parse_full_config() {
    let config: Config = toml::from_str(FULL_TOML).unwrap();
    assert_eq!(config.pack.path.as_deref(), Some("packs/mono"));
    assert_eq!(config.pack.name, "my-custom");
    assert_eq!(
        config.colors.scheme.as_deref(),
        Some("~/.themes/my.colors")
    );
    assert_eq!(config.export.sizes, vec![16, 32, 64]);
    assert!(!config.export.generate_16_symlinks);
    assert_eq!(config.gui.window_width, 1024);
    assert_eq!(config.gui.window_height, 768);

    let overrides = config.colors.overrides.as_ref().unwrap();
    assert_eq!(overrides.foreground.as_deref(), Some("#ffffff"));
    assert_eq!(overrides.accent.as_deref(), Some("#ff0000"));
}

#[test]
fn default_config_values() {
    let config = Config::default();
    assert_eq!(config.pack.name, "mono");
    assert!(config.pack.path.is_none());
    assert!(config.colors.scheme.is_none());
    assert!(config.export.generate_16_symlinks);
    assert_eq!(config.export.sizes.len(), 9);
    assert_eq!(config.gui.window_width, 800);
}

#[test]
fn empty_toml_uses_defaults() {
    let config: Config = toml::from_str("").unwrap();
    assert_eq!(config.pack.name, "mono");
    assert_eq!(config.gui.window_width, 800);
}

#[test]
fn partial_pack_config() {
    let toml = r#"[pack]
path = "custom/path"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.pack.path.as_deref(), Some("custom/path"));
    assert_eq!(config.pack.name, "mono");
}

#[test]
fn load_nonexistent_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent.toml");
    let err = load(&path.to_string_lossy()).unwrap_err();
    assert!(matches!(err, ConfigError::Io(_)));
}

#[test]
fn load_valid_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.toml");
    std::fs::write(&path, MINIMAL_TOML).unwrap();
    let config = load(&path.to_string_lossy()).unwrap();
    assert_eq!(config.pack.path.as_deref(), Some("packs/mono"));
}

#[test]
fn config_paths_for_includes_home() {
    let dir = tempfile::tempdir().unwrap();
    let paths = config_paths_for(Some(dir.path()));
    assert!(paths.iter().any(|p| p.starts_with(dir.path())));
}

#[test]
fn find_config_path_returns_none_when_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let paths = config_paths_for(Some(dir.path()));
    assert!(!paths.iter().any(|p| p.is_file()));
}

#[test]
fn find_config_path_finds_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".config");
    std::fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("miconium.toml");
    std::fs::write(&config_path, "").unwrap();

    let paths = config_paths_for(Some(dir.path()));
    assert!(paths.iter().any(|p| p.is_file()));
}

#[test]
fn invalid_toml_returns_parse_error() {
    let err: Result<Config, _> = toml::from_str("[[[invalid]]]");
    assert!(err.is_err());
}
