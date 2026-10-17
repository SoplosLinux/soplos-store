// Firmware source: updates for the firmware of the computer and its
// devices (BIOS, drives, peripherals) from the LVFS, through the fwupd
// daemon and its fwupdmgr command, like GNOME Software and Discover do.
// Reading is done as the user; installing is authorized by fwupd itself
// through polkit.

use std::time::Duration;

use serde_json::Value;

use crate::system;

pub struct FirmwareUpdate {
    /// fwupd's device ID (40 hex digits).
    pub device_id: String,
    pub name: String,
    pub from: String,
    pub to: String,
}

pub fn available() -> bool {
    system::has_command("fwupdmgr")
}

/// Output of an fwupdmgr command that prints JSON. fwupdmgr exits with a
/// failure status when there is nothing to report ("no updates"), which
/// reads as None here.
fn json(args: &[&str]) -> Option<Value> {
    let out = system::output_within("fwupdmgr", args, Duration::from_secs(30))?;
    serde_json::from_str(&out).ok()
}

fn text(value: &Value, key: &str) -> String {
    value.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

pub fn updates() -> Vec<FirmwareUpdate> {
    if !available() {
        return Vec::new();
    }
    let Some(report) = json(&["get-updates", "--json", "--no-metadata-check"]) else {
        return Vec::new();
    };
    report
        .get("Devices")
        .and_then(|d| d.as_array())
        .map(|devices| {
            devices
                .iter()
                .filter_map(|device| {
                    let release = device.get("Releases")?.as_array()?.first()?;
                    let device_id = text(device, "DeviceId");
                    if device_id.is_empty() {
                        return None;
                    }
                    Some(FirmwareUpdate { device_id, name: text(device, "Name"), from: text(device, "Version"), to: text(release, "Version") })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Whether the LVFS testing channel (firmware not yet declared stable) is
/// enabled.
pub fn testing_enabled() -> bool {
    if !available() {
        return false;
    }
    json(&["get-remotes", "--json"])
        .and_then(|report| {
            report
                .get("Remotes")?
                .as_array()?
                .iter()
                .find(|remote| remote.get("Id").and_then(|id| id.as_str()) == Some("lvfs-testing"))
                .and_then(|remote| remote.get("Enabled")?.as_bool())
        })
        .unwrap_or(false)
}
