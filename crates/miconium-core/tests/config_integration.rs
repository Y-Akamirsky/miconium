use std::path::PathBuf;

use miconium_core::config::{self, Config, PackConfig};

/// A pack addressed by an explicit path that exists wins over any name lookup.
#[test]
fn explicit_existing_path_is_preferred() {
    let dir = tempfile::tempdir().unwrap();
    let pack_dir = dir.path().join("my-pack");
    std::fs::create_dir_all(&pack_dir).unwrap();

    let cfg = PackConfig {
        path: Some(pack_dir.to_string_lossy().into_owned()),
        name: "yamis".into(),
        ..PackConfig::default()
    };

    assert_eq!(
        miconium_core::pack::resolve_pack_path(&cfg),
        Some(pack_dir)
    );
}

/// A path that does not exist falls through to the name-based search, which
/// finds nothing here, so resolution reports "no pack".
#[test]
fn missing_explicit_path_falls_through_to_none() {
    let cfg = PackConfig {
        path: Some("/definitely/not/a/real/pack/xyzzy".into()),
        name: "a-pack-that-does-not-exist-anywhere".into(),
        ..PackConfig::default()
    };

    assert_eq!(miconium_core::pack::resolve_pack_path(&cfg), None);
}

/// The system share directory is always part of the search path, so a packaged
/// pack (e.g. `yamis` from miconium-iconpack) resolves without a config file.
#[test]
fn system_share_dir_is_searched() {
    let dirs = miconium_core::pack::pack_search_dirs();
    assert!(
        dirs.iter().any(|d| d == &PathBuf::from("/usr/share/miconium")),
        "expected /usr/share/miconium in the pack search dirs, got: {dirs:?}"
    );

    // The bundled pack lives there, so resolving it by name must succeed.
    let cfg = PackConfig {
        path: None,
        name: "yamis".into(),
        ..PackConfig::default()
    };
    if PathBuf::from("/usr/share/miconium/yamis").is_dir() {
        assert_eq!(
            miconium_core::pack::resolve_pack_path(&cfg),
            Some(PathBuf::from("/usr/share/miconium/yamis"))
        );
    }
}

/// The default pack name is the bundled one, so a user with no config at all
/// still resolves a real pack.
#[test]
fn default_config_resolves_bundled_pack() {
    let cfg = Config::default();
    assert_eq!(cfg.pack.name, "yamis");
    assert!(cfg.pack.path.is_none());
}

/// Config parsing keeps the pack section intact.
#[test]
fn load_pack_section_from_toml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("miconium.toml");
    std::fs::write(
        &path,
        "[pack]\nname = \"yamis\"\n\n[gui]\nwindow_width = 1024\n",
    )
    .unwrap();

    let config = config::load(&path.to_string_lossy()).unwrap();
    assert_eq!(config.pack.name, "yamis");
    assert_eq!(config.pack.path, None);
    assert_eq!(config.gui.window_width, 1024);
}