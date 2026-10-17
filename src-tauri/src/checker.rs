// Update check: `soplos-store --check-updates`, run by the systemd user
// timer soplos-store-updates.timer. It counts the pending updates
// (read-only) and stores the result in ~/.cache/soplos-store/updates.json.
// The tray only shows what is stored there; it checks by itself only when
// the timer has not run for longer than the chosen frequency.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{apt, cache, flatpak, fwupd, settings, snap};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stored {
    pub count: usize,
    /// Seconds since the Unix epoch.
    pub checked_at: u64,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn file() -> std::path::PathBuf {
    cache::dir("").join("updates.json")
}

pub fn read() -> Option<Stored> {
    serde_json::from_slice(&std::fs::read(file()).ok()?).ok()
}

pub fn store(count: usize) {
    if let Ok(data) = serde_json::to_vec(&Stored { count, checked_at: now() }) {
        let _ = std::fs::write(file(), data);
    }
}

/// How long ago the stored result was made; None when there is none.
pub fn age() -> Option<Duration> {
    read().map(|stored| Duration::from_secs(now().saturating_sub(stored.checked_at)))
}

/// Pending updates in every installed source.
pub fn count() -> usize {
    apt::invalidate_installed();
    apt::upgradable().len() + flatpak::updates().len() + snap::updates().len() + fwupd::updates().len()
}

/// Checks and stores the result.
pub fn check_and_store() -> usize {
    let found = count();
    store(found);
    found
}

/// Entry point for `--check-updates`. The timer fires every hour; the
/// check itself only happens when the stored result is older than the
/// frequency chosen in Settings.
pub fn run() -> i32 {
    let config = settings::load();
    if !config.auto_check {
        eprintln!("soplos-store check: automatic checks are off in Settings, exiting");
        return 0;
    }
    if age().is_some_and(|age| age < config.interval()) {
        return 0;
    }
    let found = check_and_store();
    eprintln!("soplos-store check: {found} updates found");
    0
}
