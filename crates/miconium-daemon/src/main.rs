//! `miconiumd` — the Miconium icon-theme daemon (item 13).
//!
//! * `miconiumd run` — watches the colour source of the configured default
//!   preset and (re)generates its icon theme on change.
//! * `miconiumd cleanup-cache [--all]` — drops expired generated
//!   `Miconium-<hash>` theme directories (never the active one), or with
//!   `--all` every generated theme **except** the active one.

use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use miconium_core::config;
use miconium_core::daemon;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = match args.get(1) {
        Some(s) => s.as_str(),
        None => "run",
    };
    match cmd {
        "run" => run_daemon(),
        "cleanup-cache" => cleanup_cache(&args[2..]),
        other => {
            eprintln!("miconiumd: unknown command '{other}'\nusage: miconiumd <run|cleanup-cache>");
            std::process::exit(2);
        }
    }
}

/// Generate (or refresh) the theme for the configured default preset, polling
/// the colour source on the configured `source_poll_seconds` interval and
/// re-reading which preset is the default every `preset_poll_seconds`. Themes
/// are applied via the active backend only when they actually change, so the
/// daemon never re-switches the icon theme on every poll.
fn run_daemon() {
    let daemon_cfg = config::load_or_default().unwrap_or_default();
    let source_poll = daemon_cfg.source_poll_seconds();
    let preset_poll = daemon_cfg.preset_poll_seconds();

    eprintln!(
        "miconiumd: starting (source poll {source_poll}s, preset poll {preset_poll}s)",
    );
    if daemon::using_dms() {
        eprintln!("miconiumd: DankMaterialShell detected — applying themes via `dms ipc`");
    } else {
        eprintln!(
            "miconiumd: DMS not detected — falling back to gsettings / gtk settings.ini"
        );
    }

    let mut active_preset: Option<String> = None;
    let mut last_preset_check = 0u64;
    // The last theme name we applied. Used to avoid re-applying a theme that is
    // already active on every poll (the reported 15s spam).
    let mut last_applied: Option<String> = None;

    loop {
        let now = now_secs();

        // Re-evaluate the default-preset selection on the slow timer (and on
        // first start / whenever we have lost the active preset).
        if active_preset.is_none() || now - last_preset_check >= preset_poll {
            last_preset_check = now;
            if let Some(name) = config::default_preset() {
                active_preset = Some(name);
            } else {
                eprintln!("miconiumd: no default preset configured; nothing to do");
                active_preset = None;
            }
        }

        if let Some(name) = &active_preset {
            match config::load_preset(name) {
                Ok(cfg) => match daemon::sync_preset(name, &cfg) {
                    Ok(Some(target)) => {
                        // Only switch the active theme when it actually changed
                        // (first run, or the colour source / preset changed).
                        if last_applied.as_deref() != Some(target.as_str()) {
                            daemon::set_active_theme(&target);
                            last_applied = Some(target);
                        }
                    }
                    Ok(None) => {}
                    Err(e) => eprintln!("miconiumd: sync failed for preset '{name}': {e}"),
                },
                Err(e) => eprintln!("miconiumd: cannot load preset '{name}': {e}"),
            }
        }

        thread::sleep(Duration::from_secs(source_poll));
    }
}

/// `miconiumd cleanup-cache [--all]`.
///
/// * no flag → remove only expired (TTL) generated themes, never the active one.
/// * `--all` → remove every generated theme **except** the active one.
fn cleanup_cache(args: &[String]) {
    let root = daemon::default_icons_root();
    let result = if args.iter().any(|a| a == "--all") {
        daemon::cleanup_all_except_active(&root)
    } else {
        let ttl = config::load_or_default().map_or(
            miconium_core::config::DEFAULT_CACHE_TTL_HOURS,
            |c| c.cache_ttl_hours(),
        );
        daemon::cleanup_cache(&root, ttl)
    };

    match result {
        Ok(n) => println!("miconiumd: removed {n} generated theme director(y/ies)"),
        Err(e) => {
            eprintln!("miconiumd: cleanup failed: {e}");
            std::process::exit(1);
        }
    }
}
