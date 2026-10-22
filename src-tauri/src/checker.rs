// Update check: `soplos-store --check-updates`, run by the systemd user
// timer soplos-store-updates.timer. It looks for the pending updates
// (read-only) and stores the whole result in
// ~/.cache/soplos-store/updates.json. That file is the one source of truth:
// the tray shows its count and the Updates page shows its list, so the two
// always agree and neither has to check again when it opens. The tray checks
// by itself only when the timer has not run for longer than the chosen
// frequency.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::models::UpdateItem;
use crate::{apt, cache, catalog, settings};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stored {
    pub count: usize,
    /// Seconds since the Unix epoch.
    pub checked_at: u64,
    /// The updates found. None in a file written before the list was kept,
    /// which counts as having no result.
    #[serde(default)]
    pub items: Option<Vec<UpdateItem>>,
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

/// Stores what a check found and returns it.
pub fn save(items: Vec<UpdateItem>) -> Stored {
    let stored = Stored { count: items.len(), checked_at: now(), items: Some(items) };
    if let Ok(data) = serde_json::to_vec(&stored) {
        let _ = std::fs::write(file(), data);
    }
    stored
}

/// How long ago the stored result was made; None when there is none.
pub fn age() -> Option<Duration> {
    read().map(|stored| Duration::from_secs(now().saturating_sub(stored.checked_at)))
}

/// Looks in every source and stores the result.
pub fn scan_and_store() -> Stored {
    apt::invalidate_installed();
    save(catalog::scan().into_iter().flat_map(|source| source.items).collect())
}

/// Checks and stores the result; returns how many updates there are.
pub fn check_and_store() -> usize {
    scan_and_store().count
}

/// Refreshes apt's package lists when they are older than half the check
/// interval. Half, not the whole interval: the check runs once per interval,
/// so lists refreshed by the previous check are always a little younger than
/// one interval and a full-interval test would skip the refresh every other
/// time, counting updates from lists that are almost two intervals old.
fn refresh_apt_if_stale(interval: Duration) {
    let (age_secs, _) = apt::lists_status();
    let is_stale = match age_secs {
        Some(secs) => Duration::from_secs(secs) >= interval / 2,
        None => true,
    };
    if is_stale {
        if let Ok(exe) = std::env::current_exe() {
            match std::process::Command::new("pkexec").arg(exe).arg("--privileged-refresh").status() {
                Ok(status) if status.success() => {}
                Ok(status) => eprintln!("soplos-store check: the package lists were not refreshed ({status})"),
                Err(error) => eprintln!("soplos-store check: could not run pkexec: {error}"),
            }
        }
    }
}

/// The check of the timer and of the tray: the package lists are brought up
/// to date first when they are stale, so the count includes what Debian
/// published since, and then every source is looked in and stored.
pub fn refresh_and_check(interval: Duration) -> usize {
    refresh_apt_if_stale(interval);
    check_and_store()
}

/// Entry point for `--check-updates`. Controlled by the systemd user timer
/// soplos-store-updates.timer matching the interval configured in Settings.
pub fn run() -> i32 {
    let config = settings::load();
    if !config.auto_check {
        eprintln!("soplos-store check: automatic checks are off in Settings, exiting");
        return 0;
    }
    if age().is_some_and(|age| age < Duration::from_secs(300)) {
        return 0;
    }
    let found = refresh_and_check(config.interval());
    eprintln!("soplos-store check: {found} updates found");
    0
}
