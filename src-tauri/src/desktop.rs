// Desktop entry discovery: parse .desktop files from /usr/share/applications
// to find installed GUI applications that may not be in the DEP-11 AppStream
// index. This ensures that apps like LibreOffice, KMail or GIMP always show
// up in "Installed" even on systems whose APT metadata is incomplete.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::system;

/// A desktop application discovered from its .desktop file.
pub struct DesktopEntry {
    pub name: String,
    pub comment: String,
    pub icon: Option<String>,
}

/// Map from APT package name → DesktopEntry, built by reading every
/// .desktop file in /usr/share/applications and mapping them back to
/// their owning package with `dpkg -S`.
pub fn desktop_apps() -> HashMap<String, DesktopEntry> {
    let dir = Path::new("/usr/share/applications");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return HashMap::new();
    };

    // Collect all .desktop file paths that represent visible applications.
    let mut files: Vec<(String, DesktopEntry)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Some(app) = parse_desktop(&content) {
            files.push((path.to_string_lossy().into_owned(), app));
        }
    }

    if files.is_empty() {
        return HashMap::new();
    }

    // Ask dpkg which package owns each .desktop file, all at once.
    let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    let owners = dpkg_owners(&paths);

    // Build the result: one entry per package (first .desktop wins).
    let mut result: HashMap<String, DesktopEntry> = HashMap::new();
    for (path, entry) in files.into_iter() {
        if let Some(package) = owners.get(&path) {
            result.entry(package.clone()).or_insert(entry);
        }
    }
    result
}

/// Parse a single .desktop file and return a DesktopEntry if it is a
/// visible application (Type=Application, not NoDisplay, not Hidden).
fn parse_desktop(content: &str) -> Option<DesktopEntry> {
    let lang = system::language();
    let mut in_desktop_entry = false;
    let mut type_ok = false;
    let mut no_display = false;
    let mut hidden = false;
    let mut name = String::new();
    let mut name_localized = String::new();
    let mut comment = String::new();
    let mut comment_localized = String::new();
    let mut icon = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            if in_desktop_entry {
                break; // Only parse [Desktop Entry], stop at the next group.
            }
            if line == "[Desktop Entry]" {
                in_desktop_entry = true;
            }
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        if let Some(val) = line.strip_prefix("Type=") {
            type_ok = val.trim() == "Application";
        } else if line == "NoDisplay=true" {
            no_display = true;
        } else if line == "Hidden=true" {
            hidden = true;
        } else if let Some(val) = line.strip_prefix("Name=") {
            name = val.trim().to_string();
        } else if let Some(val) = strip_localized_key(line, "Name", &lang) {
            name_localized = val.to_string();
        } else if let Some(val) = line.strip_prefix("Comment=") {
            comment = val.trim().to_string();
        } else if let Some(val) = strip_localized_key(line, "Comment", &lang) {
            comment_localized = val.to_string();
        } else if let Some(val) = line.strip_prefix("Icon=") {
            icon = val.trim().to_string();
        }
    }

    if !type_ok || no_display || hidden || name.is_empty() {
        return None;
    }

    let final_name = if name_localized.is_empty() { name } else { name_localized };
    let final_comment = if comment_localized.is_empty() { comment } else { comment_localized };
    let resolved_icon = resolve_icon(&icon);

    Some(DesktopEntry {
        name: final_name,
        comment: final_comment,
        icon: resolved_icon,
    })
}

/// Check for a localized key like `Name[es]=...` or `Name[es_ES]=...`.
fn strip_localized_key<'a>(line: &'a str, key: &str, lang: &str) -> Option<&'a str> {
    // Try exact match first (e.g. Name[es_ES]=), then language only (Name[es]=).
    let exact = format!("{key}[{lang}]=");
    if let Some(val) = line.strip_prefix(&exact) {
        return Some(val.trim());
    }
    if let Some(short) = lang.split('_').next() {
        let prefix = format!("{key}[{short}]=");
        if let Some(val) = line.strip_prefix(&prefix) {
            return Some(val.trim());
        }
    }
    None
}

/// Resolve a theme icon name (e.g. "gimp", "libreoffice-writer") to a
/// data URI by searching the standard icon directories, just like
/// flatpak::icon() does.
pub fn resolve_icon(icon_name: &str) -> Option<String> {
    if icon_name.is_empty() {
        return None;
    }
    // If the icon field is already an absolute path, use it directly.
    if icon_name.starts_with('/') {
        let path = Path::new(icon_name);
        if path.is_file() {
            return system::file_data_uri(path);
        }
        return None;
    }

    let base = Path::new("/usr/share/icons/hicolor");
    let sizes = ["128x128", "256x256", "64x64", "48x48", "512x512", "scalable"];
    let extensions = ["png", "svg"];

    for size in &sizes {
        for ext in &extensions {
            // scalable only makes sense with svg
            if *size == "scalable" && *ext != "svg" {
                continue;
            }
            let path = base.join(size).join("apps").join(format!("{icon_name}.{ext}"));
            if path.is_file() {
                return system::file_data_uri(&path);
            }
        }
    }

    // Fallback: /usr/share/pixmaps
    let pixmaps = Path::new("/usr/share/pixmaps");
    for ext in &["png", "svg", "xpm"] {
        let path = pixmaps.join(format!("{icon_name}.{ext}"));
        if path.is_file() {
            if *ext == "xpm" {
                continue; // xpm is not supported by file_data_uri, skip
            }
            return system::file_data_uri(&path);
        }
    }

    None
}

/// Run `dpkg -S` on a list of file paths to find which package owns each.
/// Returns a map from file path → package name.
fn dpkg_owners(paths: &[&str]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if paths.is_empty() {
        return map;
    }
    // dpkg -S can take many paths at once; each output line is
    // "package: /path/to/file" or "package:arch: /path/to/file".
    // Note: dpkg -S exits with code 1 if even one file has no owner, but
    // it still outputs matches to stdout. We read stdout directly.
    let Ok(output) = std::process::Command::new("dpkg")
        .arg("-S")
        .args(paths)
        .env("LC_ALL", "C")
        .output()
    else {
        return map;
    };
    let out = String::from_utf8_lossy(&output.stdout);
    let asked: HashSet<&str> = paths.iter().copied().collect();
    for line in out.lines() {
        let Some((pkg_part, file_part)) = line.split_once(": ") else {
            continue;
        };
        let file = file_part.trim();
        if !asked.contains(file) {
            continue;
        }
        // pkg_part can be "package" or "package:amd64" or "pkg1, pkg2: /path"
        // (diversion). Take the first package, strip the arch qualifier.
        let package = pkg_part
            .split(',')
            .next()
            .unwrap_or(pkg_part)
            .trim()
            .split(':')
            .next()
            .unwrap_or(pkg_part)
            .trim();
        map.insert(file.to_string(), package.to_string());
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_desktop_valid() {
        let content = r#"[Desktop Entry]
Version=1.0
Type=Application
Name=GIMP
Name[es]=Editor de imágenes GIMP
Comment=Create images and edit photographs
Comment[es]=Cree imágenes y edite fotografías
Exec=gimp-2.10 %U
Icon=gimp
Terminal=false
Categories=Graphics;2DGraphics;RasterGraphics;
"#;
        let entry = parse_desktop(content);
        assert!(entry.is_some());
        let e = entry.unwrap();
        assert!(e.name.contains("GIMP"));
    }

    #[test]
    fn test_parse_desktop_hidden_or_nodisplay() {
        let content = r#"[Desktop Entry]
Type=Application
Name=Secret
NoDisplay=true
"#;
        assert!(parse_desktop(content).is_none());

        let content2 = r#"[Desktop Entry]
Type=Application
Name=HiddenApp
Hidden=true
"#;
        assert!(parse_desktop(content2).is_none());
    }
}
