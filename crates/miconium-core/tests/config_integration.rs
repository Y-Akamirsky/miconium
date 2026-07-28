use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn load_project_config() {
    let path = workspace_root().join("miconium.toml");
    let config = miconium_core::config::load(&path.to_string_lossy()).unwrap();
    assert_eq!(config.pack.name, "yamis");
    assert_eq!(config.pack.path.as_deref(), Some("packs/yamis"));
    assert_eq!(config.gui.window_width, 800);
}

#[test]
fn project_config_pack_path_exists() {
    let path = workspace_root().join("miconium.toml");
    let config = miconium_core::config::load(&path.to_string_lossy()).unwrap();
    if let Some(pack_path) = &config.pack.path {
        let full_path = workspace_root().join(pack_path);
        assert!(full_path.is_dir(), "pack path {full_path:?} does not exist");
    }
}
