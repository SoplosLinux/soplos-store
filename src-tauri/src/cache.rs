// On-disk cache under ~/.cache/soplos-store, so the app opens
// fast and still shows the last known data when there is no network.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub fn dir(sub: &str) -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(|| PathBuf::from(".cache"));
    let dir = base.join("soplos-store").join(sub);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Turns any string (usually a URL) into a safe file name.
pub fn key(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect()
}

/// File contents, only if the file is younger than `ttl`.
pub fn read_fresh(path: &Path, ttl: Duration) -> Option<Vec<u8>> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let age = SystemTime::now().duration_since(modified).unwrap_or_default();
    if age > ttl {
        return None;
    }
    std::fs::read(path).ok()
}
