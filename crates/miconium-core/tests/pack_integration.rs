use std::path::PathBuf;

use miconium_core::pack::Pack;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn load_mono_pack() {
    let path = workspace_root().join("packs/mono");
    let pack = Pack::load(&path).unwrap();
    assert_eq!(pack.name, "mono");
    assert!(pack.has_colorizable_frames());
    assert!(pack.has_static_frames());
    assert!(!pack.accessories.is_empty());
    assert!(pack.sign_count() > 0);
    assert!(pack.get_signs_by_category("apps").len() >= 2);
}

#[test]
fn mono_pack_all_categories_populated() {
    let path = workspace_root().join("packs/mono");
    let pack = Pack::load(&path).unwrap();
    let all = pack.all_signs();
    for category in &[
        "apps", "categories", "devices", "emblems", "mime", "preferences", "status",
    ] {
        assert!(all.contains_key(*category), "missing category: {category}");
        assert!(!all[*category].is_empty(), "empty category: {category}");
    }
}
