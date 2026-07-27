use std::fs;
use std::path::Path;

use super::*;

const SAMPLE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect width="24" height="24"/></svg>"#;

fn create_test_pack(root: &Path) {
    let dirs = [
        "frames/colorizable",
        "signs/actions",
        "signs/apps",
        "signs/categories",
        "signs/devices",
        "signs/emblems",
        "signs/mime",
        "signs/places",
        "signs/preferences",
        "signs/status",
    ];
    for d in &dirs {
        fs::create_dir_all(root.join(d)).unwrap();
    }

    fs::write(root.join("frames/colorizable/frame1.svg"), SAMPLE_SVG).unwrap();
    fs::write(root.join("signs/apps/firefox.svg"), SAMPLE_SVG).unwrap();
    fs::write(root.join("signs/apps/terminal.svg"), SAMPLE_SVG).unwrap();
    fs::write(root.join("signs/status/battery.svg"), SAMPLE_SVG).unwrap();
}

#[test]
fn load_valid_pack() {
    let dir = tempfile::tempdir().unwrap();
    create_test_pack(dir.path());

    let pack = Pack::load(dir.path()).unwrap();
    assert_eq!(pack.name, dir.path().file_name().unwrap().to_str().unwrap());
    assert_eq!(pack.frames.colorizable.len(), 1);
    assert!(pack.frames.static_frames.is_none());
    assert_eq!(
        pack.signs.categories.get("apps")
            .and_then(|v| v.get("scalable"))
            .map_or(0, Vec::len),
        2
    );
    assert_eq!(
        pack.signs.categories.get("status")
            .and_then(|v| v.get("scalable"))
            .map_or(0, Vec::len),
        1
    );
    assert!(pack.accessories.is_empty());
}

#[test]
fn load_missing_directory() {
    let err = Pack::load("/nonexistent/path").unwrap_err();
    assert!(matches!(err, PackError::NotFound(_)));
}

#[test]
fn load_missing_frames() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("signs/apps")).unwrap();
    fs::write(dir.path().join("signs/apps/test.svg"), SAMPLE_SVG).unwrap();

    let err = Pack::load(dir.path()).unwrap_err();
    assert!(matches!(err, PackError::MissingRequired(_)));
}

#[test]
fn load_missing_signs() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("frames/colorizable")).unwrap();
    fs::write(
        dir.path().join("frames/colorizable/test.svg"),
        SAMPLE_SVG,
    )
    .unwrap();

    let err = Pack::load(dir.path()).unwrap_err();
    assert!(matches!(err, PackError::MissingRequired(_)));
}

#[test]
fn load_optional_directories_graceful() {
    let dir = tempfile::tempdir().unwrap();
    create_test_pack(dir.path());

    fs::create_dir_all(dir.path().join("frames/static")).unwrap();
    let pack = Pack::load(dir.path()).unwrap();
    assert!(pack.frames.static_frames.is_none());
    assert!(pack.accessories.is_empty());
}

#[test]
fn load_with_accessories() {
    let dir = tempfile::tempdir().unwrap();
    create_test_pack(dir.path());
    fs::create_dir_all(dir.path().join("accessories")).unwrap();
    fs::write(dir.path().join("accessories/glow.svg"), SAMPLE_SVG).unwrap();

    let pack = Pack::load(dir.path()).unwrap();
    assert_eq!(pack.accessories.len(), 1);
}

#[test]
fn load_with_static_frames() {
    let dir = tempfile::tempdir().unwrap();
    create_test_pack(dir.path());
    fs::create_dir_all(dir.path().join("frames/static/Dark")).unwrap();
    fs::create_dir_all(dir.path().join("frames/static/Light")).unwrap();
    fs::write(
        dir.path().join("frames/static/Dark/dark_frame.svg"),
        SAMPLE_SVG,
    )
    .unwrap();
    fs::write(
        dir.path().join("frames/static/Light/light_frame.svg"),
        SAMPLE_SVG,
    )
    .unwrap();

    let pack = Pack::load(dir.path()).unwrap();
    let sf = pack.frames.static_frames.as_ref().unwrap();
    assert_eq!(sf.dark.len(), 1);
    assert_eq!(sf.light.len(), 1);
}

#[test]
fn load_with_empty_static_frames() {
    let dir = tempfile::tempdir().unwrap();
    create_test_pack(dir.path());
    fs::create_dir_all(dir.path().join("frames/static")).unwrap();

    let pack = Pack::load(dir.path()).unwrap();
    assert!(pack.frames.static_frames.is_none());
}

#[test]
fn read_svg_dir_non_existent() {
    let dir = tempfile::tempdir().unwrap();
    let result = read_svg_dir(&dir.path().join("nonexistent")).unwrap();
    assert!(result.is_empty());
}

#[test]
fn read_svg_dir_filters_non_svg() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.svg"), SAMPLE_SVG).unwrap();
    fs::write(dir.path().join("file.png"), "not svg").unwrap();
    fs::write(dir.path().join("file.txt"), "text").unwrap();

    let result = read_svg_dir(dir.path()).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].svg_content, SAMPLE_SVG);
    assert_eq!(result[0].name, "file");
}

#[test]
fn get_signs_by_category_valid() {
    let dir = tempfile::tempdir().unwrap();
    create_test_pack(dir.path());
    let pack = Pack::load(dir.path()).unwrap();

    let apps = pack.get_signs_by_category("apps");
    assert_eq!(apps.len(), 2);
    assert!(apps.iter().any(|l| l.name == "firefox"));
    assert!(apps.iter().any(|l| l.name == "terminal"));
}

#[test]
fn get_signs_by_category_invalid() {
    let dir = tempfile::tempdir().unwrap();
    create_test_pack(dir.path());
    let pack = Pack::load(dir.path()).unwrap();

    let result = pack.get_signs_by_category("invalid");
    assert!(result.is_empty());
}

#[test]
fn all_signs_returns_all_categories() {
    let dir = tempfile::tempdir().unwrap();
    create_test_pack(dir.path());
    let pack = Pack::load(dir.path()).unwrap();

    let all = pack.all_signs();
    assert_eq!(all.len(), 2);
    assert!(all.contains_key("apps"));
    assert!(all.contains_key("status"));
}

#[test]
fn sign_count() {
    let dir = tempfile::tempdir().unwrap();
    create_test_pack(dir.path());
    let pack = Pack::load(dir.path()).unwrap();

    assert_eq!(pack.sign_count(), 3);
}

#[test]
fn has_colorizable_frames() {
    let dir = tempfile::tempdir().unwrap();
    create_test_pack(dir.path());
    let pack = Pack::load(dir.path()).unwrap();

    assert!(pack.has_colorizable_frames());
    assert!(!pack.has_static_frames());
}
