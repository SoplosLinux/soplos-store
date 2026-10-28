// Snap source: the public Snap Store API for search and app data, and the
// snap command for what is installed. Only used when snapd is installed.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use crate::{http, system};

const STORE: &str = "https://api.snapcraft.io/v2/snaps/";
const HEADERS: &[(&str, &str)] = &[("Snap-Device-Series", "16")];

/// Runtime and base snaps that are not apps the user installed.
const SKIP_PREFIXES: [&str; 8] = ["core", "bare", "snapd", "gnome-", "gtk-common-themes", "kf5-", "kde-", "mesa-"];

pub struct Installed {
    pub name: String,
    pub version: String,
}

pub struct StoreSnap {
    pub name: String,
    pub title: String,
    pub summary: String,
    pub icon: Option<String>,
    /// The first screenshot the snap has, for the banners.
    pub screenshot: Option<String>,
}

/// Whether the snap command is installed.
pub fn present() -> bool {
    system::has_command("snap")
}

/// Whether the Store works with Snap: it is installed and not turned off in
/// Settings.
pub fn available() -> bool {
    present() && crate::settings::snap_enabled()
}

pub fn installed() -> Vec<Installed> {
    if !available() {
        return Vec::new();
    }
    let out = system::output("snap", &["list"]).unwrap_or_default();
    out.lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            let name = cols.first()?.to_string();
            let notes = cols.last().unwrap_or(&"");
            if notes.contains("base") || notes.contains("snapd") || SKIP_PREFIXES.iter().any(|p| name.starts_with(p)) {
                return None;
            }
            Some(Installed { name, version: cols.get(1).unwrap_or(&"").to_string() })
        })
        .collect()
}

/// Pending refreshes as (name, new version).
pub fn updates() -> Vec<(String, String)> {
    if !available() {
        return Vec::new();
    }
    let out = system::output_within("snap", &["refresh", "--list"], Duration::from_secs(20)).unwrap_or_default();
    out.lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            Some((cols.first()?.to_string(), cols.get(1).unwrap_or(&"").to_string()))
        })
        .collect()
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn media(snap: &Value, kind: &str) -> Vec<String> {
    snap.get("media")
        .and_then(|m| m.as_array())
        .map(|list| {
            list.iter()
                .filter(|m| m.get("type").and_then(|t| t.as_str()) == Some(kind))
                .filter_map(|m| m.get("url")?.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// The snaps of an answer of the "find" call of the Snap Store.
fn parse_results(data: &[u8], limit: usize) -> Vec<StoreSnap> {
    let Ok(json) = serde_json::from_slice::<Value>(data) else {
        return Vec::new();
    };
    json.get("results")
        .and_then(|r| r.as_array())
        .map(|results| {
            results
                .iter()
                .filter_map(|r| {
                    let name = r.get("name")?.as_str()?.to_string();
                    let snap = r.get("snap")?;
                    Some(StoreSnap {
                        title: snap.get("title").and_then(|t| t.as_str()).unwrap_or(&name).to_string(),
                        summary: snap.get("summary").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                        icon: media(snap, "icon").into_iter().next(),
                        screenshot: media(snap, "screenshot").into_iter().next(),
                        name,
                    })
                })
                .take(limit)
                .collect()
        })
        .unwrap_or_default()
}

pub fn search(query: &str) -> Vec<StoreSnap> {
    if !available() {
        return Vec::new();
    }
    let url = format!("{STORE}find?q={}&fields=title,summary,media", encode(query));
    http::get(&url, HEADERS).map(|data| parse_results(&data, 20)).unwrap_or_default()
}

/// The snaps of one category of the Snap Store ("featured", "games"...), in
/// the order the store gives them. Empty without snapd or when the store
/// does not answer.
pub fn category(name: &str) -> Vec<StoreSnap> {
    if !available() {
        return Vec::new();
    }
    let url = format!("{STORE}find?category={}&fields=title,summary,media", encode(name));
    http::cached(&url, "snap", Duration::from_secs(6 * 3600), HEADERS).map(|data| parse_results(&data, 30)).unwrap_or_default()
}

/// Store data of one snap, or None when no snap has that name.
pub fn info(name: &str) -> Option<Value> {
    let url = format!("{STORE}info/{}?fields=title,summary,description,media,license,publisher,version,website,download", encode(name));
    let data = http::cached(&url, "snap", Duration::from_secs(6 * 3600), HEADERS)?;
    serde_json::from_slice(&data).ok()
}

pub fn title(info: &Value) -> String {
    info.get("snap").and_then(|s| s.get("title")).and_then(|t| t.as_str()).unwrap_or("").to_string()
}

pub fn field(info: &Value, key: &str) -> String {
    info.get("snap").and_then(|s| s.get(key)).and_then(|t| t.as_str()).unwrap_or("").to_string()
}

pub fn publisher(info: &Value) -> String {
    info.get("snap")
        .and_then(|s| s.get("publisher"))
        .and_then(|p| p.get("display-name"))
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string()
}

pub fn icon(info: &Value) -> Option<String> {
    info.get("snap").and_then(|s| media(s, "icon").into_iter().next())
}

pub fn screenshots(info: &Value) -> Vec<String> {
    info.get("snap").map(|s| media(s, "screenshot")).unwrap_or_default()
}

/// Version on the stable channel.
pub fn version(info: &Value) -> String {
    let Some(channels) = info.get("channel-map").and_then(|c| c.as_array()) else {
        return String::new();
    };
    channels
        .iter()
        .find(|c| c.get("channel").and_then(|ch| ch.get("risk")).and_then(|r| r.as_str()) == Some("stable"))
        .or_else(|| channels.first())
        .and_then(|c| c.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Download size of the stable channel, in bytes.
pub fn download_size(info: &Value) -> Option<u64> {
    let channels = info.get("channel-map")?.as_array()?;
    channels
        .iter()
        .find(|c| c.get("channel").and_then(|ch| ch.get("risk")).and_then(|r| r.as_str()) == Some("stable"))
        .or_else(|| channels.first())?
        .get("download")?
        .get("size")?
        .as_u64()
}

/// Icon of an installed snap, as a data URI.
pub fn local_icon(name: &str) -> Option<String> {
    let dir = Path::new("/snap").join(name).join("current/meta/gui");
    ["icon.png", "icon.svg"].iter().map(|f| dir.join(f)).find(|p| p.is_file()).and_then(|p| system::file_data_uri(&p))
}
