// Settings shared between the window and the tray notifier. The window
// keeps its own copy too, but writes it here on every change so the tray
// (a separate process) reads the same check frequency and notification
// choice.

use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub auto_check: bool,
    pub frequency: String,
    pub notify: bool,
    /// Whether firmware (fwupd) updates are looked for at all. Turned off
    /// when the LVFS is unreachable and its answers hold the checks up.
    pub firmware_updates: bool,
    /// Whether each source is shown at all. Turned off, the Store shows
    /// nothing of it (catalog, installed apps, updates) and does not look for
    /// its updates: the user means to use another way for that source.
    pub show_flatpak: bool,
    pub show_snap: bool,
    pub show_appimage: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            auto_check: true,
            frequency: "daily".into(),
            notify: true,
            firmware_updates: true,
            show_flatpak: true,
            show_snap: true,
            show_appimage: true,
        }
    }
}

impl Settings {
    pub fn interval(&self) -> Duration {
        let hours = match self.frequency.as_str() {
            "1h" => 1,
            "6h" => 6,
            "12h" => 12,
            "weekly" => 24 * 7,
            _ => 24,
        };
        Duration::from_secs(hours * 3600)
    }
}

fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("soplos-store").join("settings.json"))
}

pub fn load() -> Settings {
    path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default()
}

/// Whether firmware updates are turned on in Settings (the default).
pub fn firmware_enabled() -> bool {
    load().firmware_updates
}

/// Whether Flatpak is turned on in Settings (the default).
pub fn flatpak_enabled() -> bool {
    load().show_flatpak
}

/// Whether Snap is turned on in Settings (the default).
pub fn snap_enabled() -> bool {
    load().show_snap
}

/// Whether AppImage is turned on in Settings (the default).
pub fn appimage_enabled() -> bool {
    load().show_appimage
}

fn user_config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
}

fn sync_systemd_timer(auto_check: bool, frequency: &str) {
    if !auto_check {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "stop", "soplos-store-updates.timer"])
            .status();
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "disable", "soplos-store-updates.timer"])
            .status();
        return;
    }

    let interval_str = match frequency {
        "1h" => "1h",
        "6h" => "6h",
        "12h" => "12h",
        "weekly" => "7d",
        _ => "1d",
    };

    if let Some(config_dir) = user_config_dir() {
        let override_dir = config_dir
            .join("systemd")
            .join("user")
            .join("soplos-store-updates.timer.d");
        if std::fs::create_dir_all(&override_dir).is_ok() {
            let content = format!(
                "[Timer]\nOnUnitActiveSec=\nOnUnitActiveSec={}\n",
                interval_str
            );
            let _ = std::fs::write(override_dir.join("override.conf"), content);
        }
    }

    let _ = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "enable", "--now", "soplos-store-updates.timer"])
        .status();
}

#[tauri::command]
pub fn save_settings(settings: serde_json::Value) -> Result<(), String> {
    let path = path().ok_or("no home directory")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())?;

    let auto_check = settings.get("autoCheck").and_then(|v| v.as_bool()).unwrap_or(true);
    let frequency = settings.get("frequency").and_then(|v| v.as_str()).unwrap_or("daily");
    sync_systemd_timer(auto_check, frequency);

    Ok(())
}
