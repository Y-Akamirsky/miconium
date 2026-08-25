use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::color;
use crate::config::{self, Config};
use crate::export;
use crate::pack;

use thiserror::Error;

/// Prefix of daemon-generated, hash-named theme directories.
pub const THEME_PREFIX: &str = "Miconium-";

/// Marker file written inside each generated theme dir (item 13).
const META_FILE: &str = ".miconium-meta.toml";

/// Compile-time workspace root, used to locate the bundled `LICENSE`.
const WORKSPACE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("pack error: {0}")]
    Pack(#[from] pack::PackError),
    #[error("export error: {0}")]
    Export(#[from] export::ExportError),
    #[error("colour error: {0}")]
    Color(String),
    #[error("serialization error: {0}")]
    Toml(String),
    #[error("preset '{0}' has no pack path")]
    PackPathMissing(String),
}

/// Metadata persisted inside a generated theme dir so the daemon can tell
/// which colour source / preset produced it and when it was generated.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThemeMeta {
    pub preset: String,
    pub hash: String,
    pub generated_at: u64,
    pub source_path: Option<String>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Expand a leading `~` to `$HOME`. Falls back to the raw path when `$HOME`
/// is unset or the path does not start with `~`.
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit(u32::from(b >> 4), 16).unwrap());
        s.push(char::from_digit(u32::from(b & 0xf), 16).unwrap());
    }
    s
}

/// The icons root (`~/.local/share/icons`).
#[must_use]
pub fn default_icons_root() -> PathBuf {
    crate::pack::expand_tilde("~/.local/share/icons")
}

/// `<hash>` → `Miconium-<hash>`.
#[must_use]
pub fn theme_name(hash: &str) -> String {
    format!("{THEME_PREFIX}{hash}")
}

/// Absolute path of the generated theme dir for a given `hash`.
#[must_use]
pub fn generated_dir(icons_root: &Path, hash: &str) -> PathBuf {
    icons_root.join(theme_name(hash))
}

/// The colour-source file path the daemon should watch, if any.
fn color_source_path(config: &Config) -> Option<String> {
    config
        .colors
        .scheme
        .clone()
        .or_else(|| config.colors.matugen.clone())
        .or_else(|| config.colors.manual.clone())
}

/// A stable hash identifying the generated theme for a preset. Combines the
/// preset name, the colour-source file content (or the resolved palette when
/// no file exists), and the layer map — so distinct presets/maps/sources never
/// collide into the same theme dir.
#[must_use]
pub fn color_source_hash(preset_name: &str, config: &Config) -> Option<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"preset:");
    hasher.update(preset_name.as_bytes());
    hasher.update(b"\n");

    let mut hashed_source = false;
    if let Some(raw) = color_source_path(config) {
        let expanded = crate::pack::expand_tilde(&raw);
        if let Ok(bytes) = std::fs::read(&expanded) {
            hasher.update(b"source:");
            hasher.update(&bytes);
            hashed_source = true;
        }
    }
    if !hashed_source {
        match color::resolve_palette(&config.colors) {
            Ok(p) => {
                hasher.update(b"palette:");
                hasher.update(p.hash_input().as_bytes());
            }
            Err(_) => return None,
        }
    }

    // Include the layer map so different colour→layer mappings produce
    // distinct themes even with the same source.
    hasher.update(b"map:");
    hasher.update(config.colors.map.frame.as_bytes());
    hasher.update(b",");
    hasher.update(config.colors.map.sign.as_bytes());
    hasher.update(b",");
    hasher.update(config.colors.map.accessory.as_bytes());

    Some(hex_encode(&hasher.finalize()))
}

/// Persist [`ThemeMeta`] inside the generated theme dir.
pub fn write_meta(dir: &Path, meta: &ThemeMeta) -> Result<(), DaemonError> {
    std::fs::create_dir_all(dir)?;
    let toml = toml::to_string_pretty(meta).map_err(|e| DaemonError::Toml(e.to_string()))?;
    std::fs::write(dir.join(META_FILE), toml)?;
    Ok(())
}

/// Read the [`ThemeMeta`] from a generated theme dir, if present and valid.
#[must_use]
pub fn read_meta(dir: &Path) -> Option<ThemeMeta> {
    let content = std::fs::read_to_string(dir.join(META_FILE)).ok()?;
    toml::from_str(&content).ok()
}

/// Whether the theme for `hash` must be (re)generated: the dir is missing, or
/// its stored hash no longer matches (colour source / preset changed).
#[must_use]
pub fn needs_regeneration(dir: &Path, hash: &str) -> bool {
    if !dir.is_dir() {
        return true;
    }
    match read_meta(dir) {
        // Dir exists but carries no meta → treat as stale.
        Some(meta) => meta.hash != hash,
        None => true,
    }
}

fn run_quiet(program: &str, args: &[&str]) {
    let _ = std::process::Command::new(program).args(args).status();
}

/// Locate the `dms` binary, if installed.
fn dms_binary() -> Option<PathBuf> {
    for candidate in ["/usr/bin/dms", "/usr/local/bin/dms", "/bin/dms"] {
        if Path::new(candidate).is_file() {
            return Some(PathBuf::from(candidate));
        }
    }
    // Fall back to a PATH lookup.
    if let Ok(out) = std::process::Command::new("which").arg("dms").output() {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

/// Whether a `dms` / `DankMaterialShell` process is currently running (Linux
/// only; scans `/proc`). Used to decide whether themes can be applied via the
/// DMS IPC.
fn dms_running() -> bool {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return false;
    };
    for entry in entries.flatten() {
        let comm = entry.path().join("comm");
        if let Ok(name) = std::fs::read_to_string(&comm) {
            let name = name.trim();
            if name == "dms" || name == "DankMaterialShell" {
                return true;
            }
        }
    }
    false
}

/// Whether the daemon should drive icon themes through `DankMaterialShell`'s IPC
/// (DMS binary present **and** a DMS process running).
#[must_use]
pub fn using_dms() -> bool {
    dms_binary().is_some() && dms_running()
}

fn run_dms_ipc(bin: &Path, key: &str, value: &str) {
    let status = std::process::Command::new(bin)
        .args(["ipc", "call", "settings", "set", key, value])
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!("miconiumd: dms ipc set {key} exited with {s}"),
        Err(e) => eprintln!("miconiumd: dms ipc call failed: {e}"),
    }
}

/// Point the DE at `name` as the active icon theme.
///
/// When `DankMaterialShell` is present and running, themes are applied **only**
/// via `dms ipc call settings set …` (instant, live reload). Otherwise the
/// daemon falls back to gsettings + the gtk-3.0 / gtk-4.0 `settings.ini`
/// files. All failures are logged, never fatal.
pub fn set_active_theme(name: &str) {
    if let Some(bin) = dms_binary() {
        if dms_running() {
            run_dms_ipc(&bin, "iconThemeDark", name);
            run_dms_ipc(&bin, "iconThemeLight", name);
            run_dms_ipc(&bin, "lastAppliedIconTheme", name);
            return;
        }
    }
    run_quiet(
        "gsettings",
        &["set", "org.gnome.desktop.interface", "icon-theme", name],
    );
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        write_gtk_ini(&home.join(".config/gtk-3.0/settings.ini"), name);
        write_gtk_ini(&home.join(".config/gtk-4.0/settings.ini"), name);
    }
}

/// Read a setting from the DMS IPC (`dms ipc call settings get <key>`),
/// returning the parsed value (quotes/whitespace stripped). `None` on any
/// failure so callers can fall back gracefully.
fn run_dms_ipc_get(bin: &Path, key: &str) -> Option<String> {
    let out = std::process::Command::new(bin)
        .args(["ipc", "call", "settings", "get", key])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let value = parse_theme_value(&String::from_utf8_lossy(&out.stdout));
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Normalise a theme-name value read from gsettings / dms / a GTK ini file:
/// drop a possible `SETTINGS_GET_SUCCESS` prefix and surrounding quotes.
fn parse_theme_value(raw: &str) -> String {
    let trimmed = raw.trim();
    let stripped = trimmed
        .strip_prefix("SETTINGS_GET_SUCCESS")
        .map_or(trimmed, str::trim);
    stripped
        .trim_matches(|c: char| c.is_whitespace() || c == '\'' || c == '"')
        .to_string()
}

/// Read `gtk-icon-theme-name` from `~/.config/gtk-3.0/settings.ini`.
fn read_gtk_ini_theme() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let path = Path::new(&home).join(".config/gtk-3.0/settings.ini");
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_settings = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_settings = trimmed.eq_ignore_ascii_case("[Settings]");
            continue;
        }
        if in_settings && trimmed.to_ascii_lowercase().starts_with("gtk-icon-theme-name") {
            if let Some((_, v)) = trimmed.split_once('=') {
                let v = parse_theme_value(v);
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// The icon theme currently selected in the active backend (DMS IPC, gsettings,
/// or the GTK `settings.ini`). Used to avoid re-applying a theme that is
/// already active. `None` when the active theme cannot be determined.
#[must_use]
pub fn active_theme_name() -> Option<String> {
    if using_dms() {
        if let Some(bin) = dms_binary() {
            if let Some(v) = run_dms_ipc_get(&bin, "iconThemeDark") {
                return Some(v);
            }
            if let Some(v) = run_dms_ipc_get(&bin, "lastAppliedIconTheme") {
                return Some(v);
            }
        }
        return None;
    }
    if let Ok(out) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "icon-theme"])
        .output()
    {
        if out.status.success() {
            let v = parse_theme_value(&String::from_utf8_lossy(&out.stdout));
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    read_gtk_ini_theme()
}

/// Merge/update `gtk-icon-theme-name` inside a GTK `settings.ini`, preserving
/// the rest of the file.
fn write_gtk_ini(path: &Path, theme_name: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    let mut in_settings = false;
    let mut replaced = false;
    for line in &mut lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_settings = trimmed.eq_ignore_ascii_case("[Settings]");
            continue;
        }
        if in_settings && trimmed.to_ascii_lowercase().starts_with("gtk-icon-theme-name") {
            *line = format!("gtk-icon-theme-name={theme_name}");
            replaced = true;
        }
    }
    if !replaced {
        if !lines
            .iter()
            .any(|l| l.trim().eq_ignore_ascii_case("[Settings]"))
        {
            if !lines.is_empty() && !lines.last().unwrap().is_empty() {
                lines.push(String::new());
            }
            lines.push("[Settings]".into());
        }
        lines.push(format!("gtk-icon-theme-name={theme_name}"));
    }
    let _ = std::fs::write(path, lines.join("\n"));
}

/// Generate the icon theme for `hash` into `dir` and mark it active.
fn regenerate(
    preset_name: &str,
    config: &Config,
    hash: &str,
    dir: &Path,
) -> Result<(), DaemonError> {
    let path = pack::resolve_pack_path(&config.pack)
        .ok_or_else(|| DaemonError::PackPathMissing(preset_name.to_string()))?;
    let pack = pack::Pack::load(&path)?;
    // Mirror the GUI: resolve the raw palette, then apply the preset's layer
    // map (Frame→background, Sign→accent, Acc→secondary_container, …) so the
    // daemon honours the per-preset colour assignment instead of the defaults.
    let palette = resolve_preset_palette(config)?;

    let mut export_cfg = config.export.clone();
    export_cfg.output = Some(dir.to_string_lossy().into());

    // The daemon does not surface progress; drain the channel.
    let (tx, _rx) = mpsc::channel();
    export::export_pack_to(
        dir,
        &pack,
        &palette,
        &export_cfg,
        &config.pack.category_overrides,
        Path::new(WORKSPACE_ROOT),
        &tx,
    )?;

    let meta = ThemeMeta {
        preset: preset_name.to_string(),
        hash: hash.to_string(),
        generated_at: now_secs(),
        source_path: color_source_path(config),
    };
    write_meta(dir, &meta)?;

    if let Err(e) = export::refresh_icon_cache(dir) {
        eprintln!("miconiumd: icon cache refresh failed: {e}");
    }
    Ok(())
}

/// Resolve the palette exactly as the GUI does: raw colour-source palette
/// with the preset's [`config::ColorsConfig::map`] applied, so Frame/Sign/Acc
/// are assigned to the configured roles (not the built-in defaults).
fn resolve_preset_palette(config: &Config) -> Result<color::Palette, DaemonError> {
    color::resolve_palette(&config.colors)
        .map_err(|e| DaemonError::Color(e.to_string()))
        .map(|p| p.with_layer_map(&config.colors.map))
}

/// Ensure the theme for `preset_name`/`config` exists (regenerating it when
/// the colour source / preset changed). Returns the generated theme name
/// (e.g. `Miconium-<hash>`) when a theme is managed, or `Ok(None)` when the
/// preset has no usable colour source and is skipped. The caller is
/// responsible for applying the theme via [`set_active_theme`] (and only when
/// it actually needs to change), so we never spam theme switches on every poll.
pub fn sync_preset(preset_name: &str, config: &Config) -> Result<Option<String>, DaemonError> {
    let Some(hash) = color_source_hash(preset_name, config) else {
        eprintln!("miconiumd: preset '{preset_name}' has no usable colour source, skipping");
        return Ok(None);
    };
    let icons_root = default_icons_root();
    let dir = generated_dir(&icons_root, &hash);

    if needs_regeneration(&dir, &hash) {
        eprintln!("miconiumd: regenerating theme for preset '{preset_name}' (hash {hash})");
        regenerate(preset_name, config, &hash, &dir)?;
    }
    Ok(Some(theme_name(&hash)))
}

/// Remove generated themes whose `generated_at` is older than `ttl_hours`.
/// `ttl_hours == 0` disables cleanup (nothing is removed). Returns the number
/// of removed theme directories. The currently active theme is never removed.
pub fn cleanup_cache(icons_root: &Path, ttl_hours: u64) -> Result<usize, DaemonError> {
    if !icons_root.is_dir() {
        return Ok(0);
    }
    let ttl = ttl_hours.saturating_mul(3600);
    let now = now_secs();
    let active = active_theme_name();
    // If we cannot determine the active theme (e.g. DMS IPC unreadable) we must
    // not risk deleting the one in use — skip cleanup entirely.
    if active.is_none() && using_dms() {
        eprintln!("miconiumd: cannot determine active theme under DMS; skipping cache cleanup");
        return Ok(0);
    }
    let mut removed = 0;
    for entry in std::fs::read_dir(icons_root)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.starts_with(THEME_PREFIX) {
            continue;
        }
        // Never remove the currently active theme.
        if active.as_deref() == Some(name) {
            continue;
        }
        if ttl_hours == 0 {
            continue;
        }
        if let Some(meta) = read_meta(&p) {
            if now.saturating_sub(meta.generated_at) > ttl {
                std::fs::remove_dir_all(&p)?;
                removed += 1;
            }
        }
    }
    Ok(removed)
}

/// Remove every generated theme directory **except** the currently active one
/// (used by `miconiumd cleanup-cache --all` and the GUI "Cleanup cache"
/// button). The active theme is kept so the live session never loses its icons.
pub fn cleanup_all_except_active(icons_root: &Path) -> Result<usize, DaemonError> {
    if !icons_root.is_dir() {
        return Ok(0);
    }
    let active = active_theme_name();
    if active.is_none() && using_dms() {
        eprintln!("miconiumd: cannot determine active theme under DMS; skipping cache cleanup");
        return Ok(0);
    }
    let mut removed = 0;
    for entry in std::fs::read_dir(icons_root)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.starts_with(THEME_PREFIX) {
            continue;
        }
        if active.as_deref() == Some(name) {
            continue;
        }
        std::fs::remove_dir_all(&p)?;
        removed += 1;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config_with_source(path: &str) -> Config {
        let mut c = Config::default();
        c.colors.manual = Some(path.to_string());
        c
    }

    #[test]
    fn hash_is_stable_and_content_sensitive() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("colors.txt");
        std::fs::write(&f, "aabbcc").unwrap();
        let c = test_config_with_source(&f.to_string_lossy());
        let h1 = color_source_hash("p1", &c).unwrap();
        let h2 = color_source_hash("p1", &c).unwrap();
        assert_eq!(h1, h2);
        // Different content → different hash.
        std::fs::write(&f, "ddeeff").unwrap();
        let h3 = color_source_hash("p1", &c).unwrap();
        assert_ne!(h1, h3);
        // Different preset name → different hash (avoids collisions).
        let h4 = color_source_hash("p2", &c).unwrap();
        assert_ne!(h1, h4);
    }

    #[test]
    fn theme_naming() {
        assert_eq!(theme_name("abc"), "Miconium-abc");
        let root = Path::new("/tmp/icons");
        assert_eq!(generated_dir(root, "abc"), root.join("Miconium-abc"));
    }

    #[test]
    fn meta_roundtrip_and_regen_flag() {
        let dir = tempfile::tempdir().unwrap();
        let meta = ThemeMeta {
            preset: "p".into(),
            hash: "h1".into(),
            generated_at: 123,
            source_path: None,
        };
        write_meta(dir.path(), &meta).unwrap();
        let read = read_meta(dir.path()).unwrap();
        assert_eq!(read.hash, "h1");
        assert!(!needs_regeneration(dir.path(), "h1"));
        assert!(needs_regeneration(dir.path(), "h2"));
        assert!(needs_regeneration(&dir.path().join("nope"), "h1"));
    }

    #[test]
    fn cleanup_removes_only_expired() {
        let root = tempfile::tempdir().unwrap();
        let now = now_secs();
        let old = generated_dir(root.path(), "old");
        std::fs::create_dir_all(&old).unwrap();
        write_meta(
            &old,
            &ThemeMeta {
                preset: "p".into(),
                hash: "old".into(),
                generated_at: now - 10_000,
                source_path: None,
            },
        )
        .unwrap();
        let fresh = generated_dir(root.path(), "fresh");
        std::fs::create_dir_all(&fresh).unwrap();
        write_meta(
            &fresh,
            &ThemeMeta {
                preset: "p".into(),
                hash: "fresh".into(),
                generated_at: now,
                source_path: None,
            },
        )
        .unwrap();
        // Unrelated dir must be left alone.
        let other = root.path().join("Miconiumx-not-theme");
        std::fs::create_dir_all(&other).unwrap();

        let removed = cleanup_cache(root.path(), 1).unwrap();
        assert_eq!(removed, 1);
        assert!(!old.is_dir());
        assert!(fresh.is_dir());
        assert!(other.is_dir());
    }

    #[test]
    fn cleanup_disabled_when_ttl_zero() {
        let root = tempfile::tempdir().unwrap();
        let now = now_secs();
        let old = generated_dir(root.path(), "old");
        std::fs::create_dir_all(&old).unwrap();
        write_meta(
            &old,
            &ThemeMeta {
                preset: "p".into(),
                hash: "old".into(),
                generated_at: now - 10_000,
                source_path: None,
            },
        )
        .unwrap();
        let removed = cleanup_cache(root.path(), 0).unwrap();
        assert_eq!(removed, 0);
        assert!(old.is_dir());
    }

    #[test]
    fn preset_layer_map_is_applied_to_palette() {
        // Manual colour source with three distinct roles.
        let dir = tempfile::tempdir().unwrap();
        let colors = dir.path().join("manual.toml");
        std::fs::write(
            &colors,
            "[manual]\ntop = \"#aabbcc\"\nbottom = \"#112233\"\n",
        )
        .unwrap();

        let mut cfg = Config::default();
        cfg.colors.manual = Some(colors.to_string_lossy().into());
        // Frame → background, Sign → accent, Acc → secondary_container.
        cfg.colors.map.frame = "background".into();
        cfg.colors.map.sign = "accent".into();
        cfg.colors.map.accessory = "secondary_container".into();
        // Provide the `secondary_container` role in the resolved palette.
        cfg.colors.overrides = Some(crate::config::ColorOverrides {
            foreground: Some("#dddddd".into()),
            background: Some("#222222".into()),
            accent: Some("#eeeeee".into()),
        });

        let palette = resolve_preset_palette(&cfg).unwrap();
        // With the custom map, the engine injects `background` into the frame,
        // `accent` into the sign, and `secondary_container` (no such role →
        // unchanged) into the accessory. The key check is that `foreground`
        // is the *accent* colour, proving the map was applied (the default map
        // would have left it as the foreground colour `#dddddd`).
        assert_eq!(palette.background, "#222222");
        assert_eq!(palette.foreground, "#eeeeee");
        assert_eq!(palette.accent, "#eeeeee");
    }
}

