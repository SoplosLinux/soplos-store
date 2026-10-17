// APT source: Debian's AppStream metadata (DEP-11), which apt already
// downloads to /var/lib/apt/lists with every "apt update", plus dpkg and
// apt for what is installed and what can be upgraded.

use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use serde_yaml::Value;

use crate::models::{Link, ReleaseInfo};
use crate::{odrs, system, text};

const LISTS_DIR: &str = "/var/lib/apt/lists";

#[derive(Clone, Default)]
pub struct AptApp {
    pub id: String,
    pub package: String,
    pub name: String,
    pub summary: String,
    pub description: Vec<String>,
    pub developer: String,
    pub license: String,
    pub homepage: String,
    pub icon: Option<String>,
    pub screenshots: Vec<String>,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub version: String,
    /// Desktop file names without ".desktop", e.g. "thunderbird".
    pub desktop_ids: Vec<String>,
    pub links: Vec<Link>,
    pub release: Option<ReleaseInfo>,
}

/// Everything read from the DEP-11 files, with the lookup tables built on
/// top of it. Built once and kept for the whole run; parsing all the files
/// takes a few seconds, so it is started in the background when the app
/// opens. After a refresh of the package lists it is built again.
pub struct Database {
    pub apps: HashMap<String, AptApp>,
    by_package: HashMap<String, String>,
    /// Other keys an app can be found by: its ID in lowercase and its
    /// desktop file names.
    ids: HashMap<String, String>,
    /// Exact app names, only when no other app shares the name.
    names: HashMap<String, String>,
}

static DATABASE: Mutex<Option<&'static Database>> = Mutex::new(None);

fn build_database() -> Database {
    let apps = build_index();
    let by_package = apps.values().map(|app| (app.package.clone(), app.id.clone())).collect();
    let mut ids = HashMap::new();
    let mut names: HashMap<String, Option<String>> = HashMap::new();
    for app in apps.values() {
        ids.insert(app.id.to_lowercase(), app.id.clone());
        for desktop in &app.desktop_ids {
            ids.entry(desktop.to_lowercase()).or_insert_with(|| app.id.clone());
        }
        let name = normalize_name(&app.name);
        if name.len() >= 4 {
            names.entry(name).and_modify(|existing| *existing = None).or_insert_with(|| Some(app.id.clone()));
        }
    }
    Database { apps, by_package, ids, names: names.into_iter().filter_map(|(k, v)| v.map(|id| (k, id))).collect() }
}

/// The current database. Each rebuild is kept alive for the rest of the
/// run (refreshes are rare, so the old copy is simply left behind).
fn db() -> &'static Database {
    let mut guard = DATABASE.lock().unwrap_or_else(|e| e.into_inner());
    *guard.get_or_insert_with(|| Box::leak(Box::new(build_database())))
}

/// Reads the DEP-11 files again, after the package lists were refreshed.
pub fn reload() {
    let fresh: &'static Database = Box::leak(Box::new(build_database()));
    *DATABASE.lock().unwrap_or_else(|e| e.into_inner()) = Some(fresh);
}

/// Every desktop application known to apt, by AppStream ID.
pub fn index() -> &'static HashMap<String, AptApp> {
    &db().apps
}

pub fn normalize_name(name: &str) -> String {
    name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

/// The Debian app matching an AppStream ID from another source (Flathub),
/// falling back to the desktop file name and then the exact app name,
/// since Debian and Flathub do not always use the same ID
/// (org.videolan.vlc vs org.videolan.VLC, thunderbird.desktop vs
/// org.mozilla.Thunderbird).
pub fn find(id: &str, name: &str) -> Option<&'static AptApp> {
    let db = db();
    if let Some(app) = db.apps.get(id) {
        return Some(app);
    }
    let lower = odrs::normalize(id).to_lowercase();
    let last = lower.rsplit('.').next().unwrap_or(&lower).to_string();
    [lower, last]
        .iter()
        .find_map(|key| db.ids.get(key))
        .or_else(|| db.names.get(&normalize_name(name)))
        .and_then(|real| db.apps.get(real))
}

pub fn app_for_package(package: &str) -> Option<&'static AptApp> {
    let db = db();
    db.by_package.get(package).and_then(|id| db.apps.get(id))
}

fn refreshed_stamp() -> std::path::PathBuf {
    crate::cache::dir("").join("lists-refreshed")
}

/// Records that this app just refreshed the package lists. apt gives the
/// downloaded files the date of the server, not the date of the download,
/// and Debian has no stamp of its own, so the file dates alone cannot say
/// when the lists were last refreshed.
pub fn mark_refreshed() {
    let _ = std::fs::write(refreshed_stamp(), b"");
}

/// State of the package lists: seconds since they were last refreshed
/// (None when they never were) and how many desktop apps the Debian
/// metadata holds. The most recent of this app's own stamp and the newest
/// list file is used: a file is never older than the refresh that
/// produced it, so it is a lower bound for refreshes made in a terminal.
pub fn lists_status() -> (Option<u64>, usize) {
    let newest_file = std::fs::read_dir(LISTS_DIR)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.ends_with("InRelease") || name.contains("_Packages") || name.ends_with("Release")
        })
        .filter_map(|entry| entry.metadata().ok()?.modified().ok())
        .max();
    let own_stamp = std::fs::metadata(refreshed_stamp()).ok().and_then(|m| m.modified().ok());
    let latest = [newest_file, own_stamp].into_iter().flatten().max();
    let age = latest.and_then(|at| SystemTime::now().duration_since(at).ok()).map(|d| d.as_secs());
    (age, db().apps.len())
}

/// Size of a package as (download, installed) in bytes, from apt's cache.
pub fn package_sizes(package: &str) -> Option<(u64, u64)> {
    let out = system::output("apt-cache", &["show", "--no-all-versions", package])?;
    let mut download = None;
    let mut installed = None;
    for line in out.lines() {
        if let Some(value) = line.strip_prefix("Size: ") {
            download = value.trim().parse::<u64>().ok();
        } else if let Some(value) = line.strip_prefix("Installed-Size: ") {
            installed = value.trim().parse::<u64>().ok().map(|kib| kib * 1024);
        }
    }
    Some((download?, installed?))
}

fn build_index() -> HashMap<String, AptApp> {
    let lang = system::language();
    let mut map = HashMap::new();
    let Ok(entries) = std::fs::read_dir(LISTS_DIR) else {
        return map;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.contains("_dep11_Components-") || !name.ends_with(".yml.gz") {
            continue;
        }
        let Ok(file) = std::fs::File::open(&path) else {
            continue;
        };
        let mut content = String::new();
        if flate2::read::GzDecoder::new(file).read_to_string(&mut content).is_err() {
            continue;
        }
        parse_components(&content, &lang, &mut map);
    }
    map
}

fn parse_components(content: &str, lang: &str, map: &mut HashMap<String, AptApp>) {
    let mut media_base = String::new();
    // Files without a "%YAML" line (third-party repositories) start with
    // the document marker itself, which would hide the header.
    let content = content.strip_prefix("---\n").unwrap_or(content);
    // Each component is its own YAML document; parsing them one by one
    // means a single malformed entry cannot hide the rest of the file.
    for stanza in content.split("\n---\n") {
        if stanza.starts_with('%') {
            continue;
        }
        let is_header = stanza.starts_with("File:");
        if !is_header && !stanza.starts_with("Type: desktop-application") && !stanza.contains("\nType: desktop-application") {
            continue;
        }
        let Ok(value) = serde_yaml::from_str::<Value>(stanza) else {
            continue;
        };
        if is_header {
            media_base = string(value.get("MediaBaseUrl")).trim_end_matches('/').to_string();
            continue;
        }
        let id = odrs::normalize(&string(value.get("ID")));
        let package = string(value.get("Package"));
        if id.is_empty() || package.is_empty() || map.contains_key(&id) {
            continue;
        }
        let app = AptApp {
            id: id.clone(),
            package,
            name: localized(value.get("Name"), lang),
            summary: localized(value.get("Summary"), lang),
            description: text::html_paragraphs(&localized(value.get("Description"), lang)),
            developer: {
                let name = localized(value.get("Developer").and_then(|d| d.get("name")), lang);
                if name.is_empty() { localized(value.get("DeveloperName"), lang) } else { name }
            },
            license: string(value.get("ProjectLicense")),
            homepage: string(value.get("Url").and_then(|u| u.get("homepage"))),
            icon: icon_url(&value, &media_base),
            screenshots: screenshot_urls(&value, &media_base),
            categories: strings(value.get("Categories")),
            keywords: strings(value.get("Keywords").and_then(|k| k.get(lang).or_else(|| k.get("C"))))
                .into_iter()
                .map(|k| k.to_lowercase())
                .collect(),
            version: value
                .get("Releases")
                .and_then(|r| r.as_sequence())
                .and_then(|r| r.first())
                .map(|r| string(r.get("version")))
                .unwrap_or_default(),
            desktop_ids: strings(value.get("Launchable").and_then(|l| l.get("desktop-id")))
                .into_iter()
                .map(|d| odrs::normalize(&d).to_string())
                .collect(),
            links: links(&value),
            release: release(&value, lang),
        };
        map.insert(id, app);
    }
}

/// Project links by kind: homepage, help, bugtracker, donation, source.
fn links(value: &Value) -> Vec<Link> {
    let Some(urls) = value.get("Url") else {
        return Vec::new();
    };
    [("homepage", "homepage"), ("help", "help"), ("bugtracker", "bugtracker"), ("donation", "donation"), ("vcs-browser", "source")]
        .iter()
        .filter_map(|(key, kind)| {
            let url = string(urls.get(*key));
            url.starts_with("https://").then(|| Link { kind: kind.to_string(), url })
        })
        .collect()
}

/// Latest release: version, date and its notes.
fn release(value: &Value, lang: &str) -> Option<ReleaseInfo> {
    let latest = value.get("Releases")?.as_sequence()?.first()?;
    let version = string(latest.get("version"));
    if version.is_empty() {
        return None;
    }
    let date = latest.get("unix-timestamp").and_then(|t| t.as_u64()).map(system::date_from_secs).unwrap_or_default();
    let notes = text::html_paragraphs(&localized(latest.get("description"), lang));
    Some(ReleaseInfo { version, date, notes })
}

fn string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_sequence())
        .map(|list| list.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

/// Text in the session language, falling back to the untranslated one.
fn localized(value: Option<&Value>, lang: &str) -> String {
    let Some(map) = value else {
        return String::new();
    };
    string(map.get(lang).or_else(|| map.get("C")))
}

fn icon_url(value: &Value, base: &str) -> Option<String> {
    let remote = value.get("Icon")?.get("remote")?.as_sequence()?;
    let urls: Vec<&str> = remote.iter().filter_map(|e| e.get("url").and_then(|u| u.as_str())).collect();
    let best = urls.iter().find(|u| u.contains("128x128")).or_else(|| urls.last())?;
    Some(format!("{base}/{best}"))
}

fn screenshot_urls(value: &Value, base: &str) -> Vec<String> {
    let Some(list) = value.get("Screenshots").and_then(|s| s.as_sequence()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|shot| {
            let thumbnails: Vec<&str> = shot
                .get("thumbnails")
                .and_then(|t| t.as_sequence())
                .map(|t| t.iter().filter_map(|e| e.get("url").and_then(|u| u.as_str())).collect())
                .unwrap_or_default();
            // 1248 px looks sharp in the large banners; 752 px is the fallback.
            let thumbnail = thumbnails
                .iter()
                .find(|u| u.contains("_1248x"))
                .or_else(|| thumbnails.iter().find(|u| u.contains("_752x")))
                .copied();
            let url = thumbnail.or_else(|| shot.get("source-image").and_then(|s| s.get("url")).and_then(|u| u.as_str()))?;
            Some(format!("{base}/{url}"))
        })
        .take(5)
        .collect()
}

type InstalledMap = Arc<HashMap<String, (String, u64)>>;

static INSTALLED_CACHE: Mutex<Option<(Instant, InstalledMap)>> = Mutex::new(None);

/// Forgets the cached list after an install, removal or upgrade.
pub fn invalidate_installed() {
    *INSTALLED_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Installed packages: name -> (version, installed size in KiB). Kept for
/// a few seconds so one screen does not call dpkg-query several times.
pub fn installed_packages() -> InstalledMap {
    let mut guard = INSTALLED_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, data)) = guard.as_ref() {
        if at.elapsed() < Duration::from_secs(20) {
            return data.clone();
        }
    }
    let mut map = HashMap::new();
    if let Some(out) = system::output("dpkg-query", &["-W", "-f=${db:Status-Abbrev}\t${Package}\t${Version}\t${Installed-Size}\n"]) {
        for line in out.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 4 || !parts[0].starts_with("ii") {
                continue;
            }
            map.insert(parts[1].to_string(), (parts[2].to_string(), parts[3].trim().parse().unwrap_or(0)));
        }
    }
    let data = Arc::new(map);
    *guard = Some((Instant::now(), data.clone()));
    data
}

/// Upgradable packages as (name, installed version, new version), from
/// the lists of the last "apt update".
pub fn upgradable() -> Vec<(String, String, String)> {
    let Some(out) = system::output("apt", &["list", "--upgradable"]) else {
        return Vec::new();
    };
    out.lines()
        .filter_map(|line| {
            let (name, rest) = line.split_once('/')?;
            let new_version = rest.split_whitespace().nth(1)?.to_string();
            let old_version = line.rsplit_once("from: ").map(|(_, v)| v.trim_end_matches(']').to_string()).unwrap_or_default();
            Some((name.to_string(), old_version, new_version))
        })
        .collect()
}

pub fn search(query: &str) -> Vec<&'static AptApp> {
    let q = query.to_lowercase();
    let mut found: Vec<(u8, &AptApp)> = index()
        .values()
        .filter_map(|app| {
            let name = app.name.to_lowercase();
            let score = if name == q || app.package == q {
                0
            } else if name.starts_with(&q) {
                1
            } else if name.contains(&q) || app.package.contains(&q) {
                2
            } else if app.keywords.iter().any(|k| k.contains(&q)) {
                3
            } else if app.summary.to_lowercase().contains(&q) {
                4
            } else {
                return None;
            };
            Some((score, app))
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.name.cmp(&b.1.name)));
    found.into_iter().take(40).map(|(_, app)| app).collect()
}

/// Apps whose AppStream categories include `category` (e.g. "Graphics").
pub fn in_category(category: &str) -> Vec<&'static AptApp> {
    index().values().filter(|app| app.categories.iter().any(|c| c == category)).collect()
}
