use std::path::Path;
use std::sync::mpsc;

use crate::config::ExportConfig;
use crate::export::export_pack;
use crate::pack::Pack;

fn sample_svg() -> String {
    r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect width="24" height="24" fill="currentColor"/></svg>"#.into()
}

fn setup_test_pack(dir: &Path) {
    use std::fs;
    let dirs = ["frames/colorizable", "signs/apps", "signs/status"];
    for d in &dirs {
        fs::create_dir_all(dir.join(d)).unwrap();
    }
    fs::write(dir.join("frames/colorizable/default.svg"), sample_svg()).unwrap();
    fs::write(dir.join("signs/apps/firefox.svg"), sample_svg()).unwrap();
    fs::write(dir.join("signs/status/battery.svg"), sample_svg()).unwrap();
}

fn test_export_config(output: String) -> ExportConfig {
    ExportConfig {
        output: Some(output),
        sizes: vec![],
        generate_16_symlinks: false,
    }
}

#[test]
fn export_to_tempdir() {
    let pack_dir = tempfile::tempdir().unwrap();
    setup_test_pack(pack_dir.path());

    let out_dir = tempfile::tempdir().unwrap();
    let project_root = tempfile::tempdir().unwrap();
    std::fs::write(project_root.path().join("index.theme"), "[Icon Theme]\nName=Miconium\n").unwrap();

    let pack = Pack::load(pack_dir.path()).unwrap();
    let palette = crate::color::Palette::default();
    let export_cfg = test_export_config(out_dir.path().to_string_lossy().to_string());

    let (tx, rx) = mpsc::channel();
    export_pack(&pack, &palette, &export_cfg, project_root.path(), &tx).unwrap();

    assert!(out_dir.path().join("apps/scalable/firefox.svg").exists(), "apps/scalable/firefox.svg should exist");
    assert!(out_dir.path().join("status/scalable/battery.svg").exists(), "status/scalable/battery.svg should exist");
    assert!(out_dir.path().join("Authors").exists());
    assert!(out_dir.path().join("index.theme").exists());

    let progress: Vec<_> = rx.try_iter().collect();
    assert_eq!(progress.len(), 2);
}

#[test]
fn export_writes_valid_svg() {
    let pack_dir = tempfile::tempdir().unwrap();
    setup_test_pack(pack_dir.path());

    let out_dir = tempfile::tempdir().unwrap();
    let project_root = tempfile::tempdir().unwrap();
    std::fs::write(project_root.path().join("index.theme"), "").unwrap();

    let pack = Pack::load(pack_dir.path()).unwrap();
    let palette = crate::color::Palette::default();
    let export_cfg = test_export_config(out_dir.path().to_string_lossy().to_string());

    let (tx, _rx) = mpsc::channel();
    export_pack(&pack, &palette, &export_cfg, project_root.path(), &tx).unwrap();

    let svg_path = out_dir.path().join("apps/scalable/firefox.svg");
    assert!(svg_path.exists(), "{:?} does not exist", svg_path);
    let content = std::fs::read_to_string(&svg_path).unwrap();
    assert!(content.starts_with("<svg"));
    assert!(content.contains("</svg>"));
    assert!(content.contains(r#"id="layer-0""#));
}

#[test]
fn export_no_frames_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("frames/colorizable")).unwrap();
    std::fs::create_dir_all(dir.path().join("signs/apps")).unwrap();
    std::fs::write(dir.path().join("signs/apps/f.svg"), sample_svg()).unwrap();

    let pack = Pack::load(dir.path()).unwrap();
    let palette = crate::color::Palette::default();
    let out_dir = tempfile::tempdir().unwrap();
    let project_root = tempfile::tempdir().unwrap();
    let export_cfg = test_export_config(out_dir.path().to_string_lossy().to_string());

    let (tx, _rx) = mpsc::channel();
    let err = export_pack(&pack, &palette, &export_cfg, project_root.path(), &tx).unwrap_err();
    assert!(matches!(err, crate::export::ExportError::NoFrames));
}
