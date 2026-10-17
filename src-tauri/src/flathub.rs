// Flathub source: the public Flathub API (collections, app picks,
// AppStream data and search). Lists are cached for 6 hours, app data
// for a day.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::models::{Link, ReleaseInfo};
use crate::{http, system, text};

const API: &str = "https://flathub.org/api/v2/";
const LIST_TTL: Duration = Duration::from_secs(6 * 3600);
const APP_TTL: Duration = Duration::from_secs(24 * 3600);

#[derive(Clone)]
pub struct Hit {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub icon: Option<String>,
    /// AppStream categories (e.g. "Audio", "Player").
    pub categories: Vec<String>,
}

pub fn str_field(value: &Value, key: &str) -> String {
    value.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn parse_hits(json: &Value) -> Vec<Hit> {
    let Some(hits) = json.get("hits").and_then(|h| h.as_array()) else {
        return Vec::new();
    };
    hits.iter()
        .filter(|h| h.get("type").and_then(|t| t.as_str()).map_or(true, |t| t == "desktop-application"))
        .filter_map(|h| {
            Some(Hit {
                id: h.get("app_id")?.as_str()?.to_string(),
                name: str_field(h, "name"),
                summary: str_field(h, "summary"),
                icon: h.get("icon").and_then(|i| i.as_str()).map(String::from),
                categories: string_list(h.get("sub_categories")),
            })
        })
        .collect()
}

/// A Flathub collection such as "collection/popular" or
/// "collection/category/graphics".
pub fn collection(path: &str, per_page: u32) -> Vec<Hit> {
    let url = format!("{API}{path}?page=1&per_page={per_page}");
    http::cached_json(&url, "flathub", LIST_TTL).map(|json| parse_hits(&json)).unwrap_or_default()
}

pub fn search(query: &str) -> Vec<Hit> {
    http::post_json(&format!("{API}search"), &serde_json::json!({ "query": query }))
        .map(|json| parse_hits(&json))
        .unwrap_or_default()
}

/// Full AppStream data of one app.
pub fn app(id: &str) -> Option<Value> {
    http::cached_json(&format!("{API}appstream/{id}"), "flathub", APP_TTL)
}

pub fn hit_from_app(id: &str, app: &Value) -> Hit {
    Hit {
        id: id.to_string(),
        name: str_field(app, "name"),
        summary: str_field(app, "summary"),
        icon: app.get("icon").and_then(|i| i.as_str()).map(String::from),
        categories: string_list(app.get("categories")),
    }
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|list| list.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

pub fn screenshots(app: &Value) -> Vec<String> {
    let Some(list) = app.get("screenshots").and_then(|s| s.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|shot| {
            let sizes = shot.get("sizes")?.as_array()?;
            // The 1248 px version looks sharp in the large banners and on the
            // app page; 752 px is the fallback.
            let by_width = |width: &str| sizes.iter().find(|s| s.get("width").and_then(|w| w.as_str()) == Some(width));
            let pick = by_width("1248").or_else(|| by_width("752")).or_else(|| sizes.first())?;
            pick.get("src")?.as_str().map(String::from)
        })
        .take(5)
        .collect()
}

pub fn version(app: &Value) -> String {
    app.get("releases")
        .and_then(|r| r.as_array())
        .and_then(|r| r.first())
        .map(|r| str_field(r, "version"))
        .unwrap_or_default()
}

pub fn description(app: &Value) -> Vec<String> {
    text::html_paragraphs(&str_field(app, "description"))
}

pub fn homepage(app: &Value) -> String {
    app.get("urls").map(|u| str_field(u, "homepage")).unwrap_or_default()
}

/// The app Flathub picked for today.
pub fn app_of_the_day() -> Option<String> {
    let url = format!("{API}app-picks/app-of-the-day/{}", date(0));
    http::cached_json(&url, "flathub", LIST_TTL)?.get("app_id")?.as_str().map(String::from)
}

/// IDs picked by the Flathub team as apps of the week, `days_ago` days
/// back (0 for this week, 7 for the week before).
pub fn apps_of_the_week(days_ago: u64) -> Vec<String> {
    let url = format!("{API}app-picks/apps-of-the-week/{}", date(days_ago));
    let Some(json) = http::cached_json(&url, "flathub", LIST_TTL) else {
        return Vec::new();
    };
    json.get("apps")
        .and_then(|a| a.as_array())
        .map(|apps| apps.iter().filter_map(|a| a.get("app_id")?.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

/// The date `days_ago` days back as YYYY-MM-DD (UTC).
fn date(days_ago: u64) -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    system::date_from_secs(now.saturating_sub(days_ago * 86_400))
}

/// Download and installed size of the Flatpak, in bytes.
pub fn sizes(id: &str) -> Option<(u64, u64)> {
    let json = http::cached_json(&format!("{API}summary/{id}"), "flathub", APP_TTL)?;
    Some((json.get("download_size")?.as_u64()?, json.get("installed_size")?.as_u64()?))
}

/// Project links by kind: homepage, help, bugtracker, donation, source.
pub fn links(app: &Value) -> Vec<Link> {
    let Some(urls) = app.get("urls") else {
        return Vec::new();
    };
    [("homepage", "homepage"), ("help", "help"), ("bugtracker", "bugtracker"), ("donation", "donation"), ("vcs_browser", "source")]
        .iter()
        .filter_map(|(key, kind)| {
            let url = urls.get(*key)?.as_str()?;
            url.starts_with("https://").then(|| Link { kind: kind.to_string(), url: url.to_string() })
        })
        .collect()
}

pub fn categories(app: &Value) -> Vec<String> {
    app.get("categories")
        .and_then(|c| c.as_array())
        .map(|list| list.iter().filter_map(|c| c.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

/// Latest release: version, date and its notes.
pub fn release(app: &Value) -> Option<ReleaseInfo> {
    let latest = app.get("releases")?.as_array()?.first()?;
    let version = str_field(latest, "version");
    if version.is_empty() {
        return None;
    }
    // The timestamp comes as a number or as a string.
    let seconds = latest
        .get("timestamp")
        .and_then(|t| t.as_u64().or_else(|| t.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0);
    Some(ReleaseInfo {
        version,
        date: if seconds > 0 { system::date_from_secs(seconds) } else { String::new() },
        notes: text::html_paragraphs(&str_field(latest, "description")),
    })
}
