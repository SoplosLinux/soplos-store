// Opening other Soplos apps and web links, and the package files the
// desktop hands to this app (.deb, .flatpak, .flatpakref, .snap,
// .AppImage) when the user double-clicks one.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;

use serde::Serialize;

use crate::system;

/// Soplos apps this one is allowed to start.
const APPS: [&str; 2] = ["soplos-repo-selector", "soplos-appimage-manager"];

const PACKAGE_EXTENSIONS: [&str; 5] = ["deb", "flatpak", "flatpakref", "snap", "appimage"];

static STARTUP_FILE: Mutex<Option<String>> = Mutex::new(None);
static STARTUP_UPDATES: Mutex<bool> = Mutex::new(false);

/// True when the app was asked to open on Updates (`--updates`, used by
/// the tray icon and its notification).
pub fn wants_updates<I: IntoIterator<Item = String>>(args: I) -> bool {
    args.into_iter().any(|arg| arg == "--updates")
}

pub fn remember_startup_updates(value: bool) {
    *STARTUP_UPDATES.lock().unwrap_or_else(|e| e.into_inner()) = value;
}

/// Whether to start on Updates; answers true only once.
#[tauri::command]
pub fn take_startup_updates() -> bool {
    std::mem::take(&mut *STARTUP_UPDATES.lock().unwrap_or_else(|e| e.into_inner()))
}

#[tauri::command]
pub fn launch_app(name: String) -> Result<(), String> {
    if !APPS.contains(&name.as_str()) {
        return Err(format!("{name} is not an allowed app"));
    }
    Command::new(&name)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("only https links can be opened".into());
    }
    Command::new("xdg-open").arg(&url).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// The package file from the command line, if the app was opened with one.
pub fn package_file_from_args<I: IntoIterator<Item = String>>(args: I) -> Option<String> {
    args.into_iter().skip(1).find(|arg| {
        let path = Path::new(arg.strip_prefix("file://").unwrap_or(arg));
        path.is_file() && extension(path).is_some_and(|ext| PACKAGE_EXTENSIONS.contains(&ext.as_str()))
    })
    .map(|arg| arg.strip_prefix("file://").unwrap_or(&arg).to_string())
}

pub fn remember_startup_file(path: Option<String>) {
    *STARTUP_FILE.lock().unwrap_or_else(|e| e.into_inner()) = path;
}

/// Returns the file the app was started with, once.
#[tauri::command]
pub fn take_startup_file() -> Option<String> {
    STARTUP_FILE.lock().unwrap_or_else(|e| e.into_inner()).take()
}

fn extension(path: &Path) -> Option<String> {
    path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase())
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalPackage {
    pub path: String,
    pub file_name: String,
    /// "deb", "flatpak", "flatpakref", "snap" or "appimage".
    pub kind: String,
    pub name: String,
    pub version: String,
    pub summary: String,
    pub size: String,
}

#[tauri::command]
pub async fn inspect_package(path: String) -> Option<LocalPackage> {
    tauri::async_runtime::spawn_blocking(move || inspect(&path)).await.ok().flatten()
}

fn inspect(path: &str) -> Option<LocalPackage> {
    let file = Path::new(path);
    let kind = extension(file).filter(|ext| PACKAGE_EXTENSIONS.contains(&ext.as_str()))?;
    let metadata = std::fs::metadata(file).ok()?;
    let file_name = file.file_name()?.to_string_lossy().into_owned();
    let mut package = LocalPackage {
        path: path.to_string(),
        name: file.file_stem()?.to_string_lossy().into_owned(),
        file_name,
        size: system::format_size(metadata.len()),
        kind: kind.clone(),
        ..Default::default()
    };
    match kind.as_str() {
        "deb" => read_deb(path, &mut package),
        "flatpakref" => read_flatpakref(path, &mut package),
        "snap" => read_snap(path, &mut package),
        // Flatpak bundles and AppImages carry no metadata that can be read
        // without installing or running them; the file name is shown.
        _ => {}
    }
    Some(package)
}

fn read_deb(path: &str, package: &mut LocalPackage) {
    let Some(out) = system::output("dpkg-deb", &["-f", path, "Package", "Version", "Description"]) else {
        return;
    };
    for line in out.lines() {
        if let Some(value) = line.strip_prefix("Package: ") {
            package.name = value.to_string();
        } else if let Some(value) = line.strip_prefix("Version: ") {
            package.version = value.to_string();
        } else if let Some(value) = line.strip_prefix("Description: ") {
            package.summary = value.to_string();
        }
    }
}

fn read_flatpakref(path: &str, package: &mut LocalPackage) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "Title" => package.name = value.trim().to_string(),
            "Name" if package.name.is_empty() || package.name == package.file_name => package.name = value.trim().to_string(),
            "Comment" | "Description" => package.summary = value.trim().to_string(),
            "Branch" => package.version = value.trim().to_string(),
            _ => {}
        }
    }
}

fn read_snap(path: &str, package: &mut LocalPackage) {
    // Needs squashfs-tools; without it only the file name is shown.
    let Some(out) = system::output("unsquashfs", &["-cat", path, "meta/snap.yaml"]) else {
        return;
    };
    for line in out.lines() {
        if let Some(value) = line.strip_prefix("name: ") {
            package.name = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("version: ") {
            package.version = value.trim().trim_matches('\'').trim_matches('"').to_string();
        } else if let Some(value) = line.strip_prefix("summary: ") {
            package.summary = value.trim().to_string();
        }
    }
}
