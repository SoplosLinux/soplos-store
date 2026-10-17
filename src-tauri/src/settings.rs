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
}

impl Default for Settings {
    fn default() -> Self {
        Settings { auto_check: true, frequency: "daily".into(), notify: true }
    }
}

impl Settings {
    pub fn interval(&self) -> Duration {
        let hours = match self.frequency.as_str() {
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

#[tauri::command]
pub fn save_settings(settings: serde_json::Value) -> Result<(), String> {
    let path = path().ok_or("no home directory")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())
}
