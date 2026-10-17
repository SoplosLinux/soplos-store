// Flatpak source: what is installed and what has updates, read with the
// flatpak command. Store data comes from Flathub (flathub.rs).

use std::collections::HashSet;
use std::path::PathBuf;

use crate::system;

pub struct Installed {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub version: String,
    pub size: String,
    /// "user" or "system".
    pub scope: String,
}

pub fn available() -> bool {
    system::has_command("flatpak")
}

pub fn installed() -> Vec<Installed> {
    if !available() {
        return Vec::new();
    }
    let Some(out) = system::output("flatpak", &["list", "--app", "--columns=application,name,description,version,size,installation"]) else {
        return Vec::new();
    };
    out.lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < 6 || cols[0].is_empty() {
                return None;
            }
            Some(Installed {
                id: cols[0].to_string(),
                name: cols[1].to_string(),
                summary: cols[2].to_string(),
                version: cols[3].to_string(),
                size: cols[4].to_string(),
                scope: cols[5].to_string(),
            })
        })
        .collect()
}

pub fn installed_ids() -> HashSet<String> {
    installed().into_iter().map(|app| app.id).collect()
}

/// Pending updates as (ID, name, new version). Includes runtimes, since
/// they are updated together with the apps.
pub fn updates() -> Vec<(String, String, String)> {
    if !available() {
        return Vec::new();
    }
    // Both installations are asked, in parallel and each with a time
    // limit (they query the remotes over the network), and the results
    // are merged.
    let per_scope = std::thread::scope(|scope| {
        let handles: Vec<_> = ["--user", "--system"]
            .iter()
            .map(|flag| {
                scope.spawn(move || {
                    system::output_within(
                        "flatpak",
                        &["remote-ls", flag, "--updates", "--columns=application,name,version"],
                        std::time::Duration::from_secs(40),
                    )
                    .unwrap_or_default()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or_default()).collect::<Vec<String>>()
    });
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for out in per_scope {
        for line in out.lines() {
            let cols: Vec<&str> = line.split('\t').collect();
            let id = cols.first().map(|s| s.trim().to_string()).unwrap_or_default();
            // Locale, Debug and Sources extensions update together with the
            // app or runtime they belong to, so they are not listed.
            let is_extension = id.ends_with(".Locale") || id.ends_with(".Debug") || id.ends_with(".Sources");
            if !crate::validate::flatpak_id(&id) || is_extension || !seen.insert(id.clone()) {
                continue;
            }
            items.push((id, cols.get(1).unwrap_or(&"").to_string(), cols.get(2).unwrap_or(&"").to_string()));
        }
    }
    items
}

/// Whether the Flathub remote exists in one installation ("--user" or
/// "--system").
pub fn has_flathub(scope_flag: &str) -> bool {
    system::output("flatpak", &["remotes", scope_flag, "--columns=name"])
        .map(|out| out.lines().any(|line| line.trim() == "flathub"))
        .unwrap_or(false)
}

/// Where the Flathub remote is configured: "user", "system", "both" or "".
pub fn flathub_scope() -> String {
    if !available() {
        return String::new();
    }
    let out = system::output("flatpak", &["remotes", "--columns=name,options"]).unwrap_or_default();
    let mut user = false;
    let mut system_wide = false;
    for line in out.lines() {
        let mut cols = line.split('\t');
        if cols.next() != Some("flathub") {
            continue;
        }
        let options = cols.next().unwrap_or("");
        if options.contains("user") {
            user = true;
        } else {
            system_wide = true;
        }
    }
    match (user, system_wide) {
        (true, true) => "both".into(),
        (true, false) => "user".into(),
        (false, true) => "system".into(),
        _ => String::new(),
    }
}

/// Icon exported by an installed Flatpak app, as a data URI.
pub fn icon(id: &str, scope: &str) -> Option<String> {
    let base = if scope == "user" {
        PathBuf::from(std::env::var_os("HOME")?).join(".local/share/flatpak/exports/share/icons/hicolor")
    } else {
        PathBuf::from("/var/lib/flatpak/exports/share/icons/hicolor")
    };
    ["128x128", "256x256", "64x64", "512x512"]
        .iter()
        .map(|size| base.join(size).join("apps").join(format!("{id}.png")))
        .chain(std::iter::once(base.join("scalable/apps").join(format!("{id}.svg"))))
        .find(|path| path.is_file())
        .and_then(|path| system::file_data_uri(&path))
}
