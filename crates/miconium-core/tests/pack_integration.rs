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
fn yamis_all_categories_populated() {
    let path = workspace_root().join("packs/yamis");
    let pack = Pack::load(&path).unwrap();
    let all = pack.all_signs();
    for category in &[
        "apps", "categories", "devices", "emblems", "mimetypes", "preferences", "status",
    ] {
        assert!(all.contains_key(*category), "missing category: {category}");
        assert!(!all[*category].is_empty(), "empty category: {category}");
    }
}

#[test]
fn load_yamis_pack() {
    let path = workspace_root().join("packs/yamis");
    let pack = Pack::load(&path).unwrap();
    assert_eq!(pack.name, "yamis");
    assert!(pack.has_colorizable_frames());
    assert!(pack.has_static_frames());
    assert!(!pack.accessories.is_empty());
    assert!(pack.sign_count() > 0);
    assert!(
        pack.get_signs_by_category("apps").len() >= 1000,
        "expected >=1000 icons in apps, got {}",
        pack.get_signs_by_category("apps").len()
    );
    let app_variants = pack.get_category_variants("apps");
    assert!(
        app_variants.contains(&"scalable".to_string()),
        "apps should have scalable variant, got: {:?}",
        app_variants
    );
    assert!(
        app_variants.contains(&"16".to_string()),
        "apps should have 16 variant, got: {:?}",
        app_variants
    );
}
