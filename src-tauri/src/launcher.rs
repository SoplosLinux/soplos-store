// Opening other Soplos apps and web links, and the package files the
// desktop hands to this app (.deb, .flatpak, .flatpakref, .snap,
// .AppImage) when the user double-clicks one.

use std::path::{Path, PathBuf};
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

/// How an installed application is started.
enum Launch {
    /// Its desktop file, started as the desktop starts it.
    Desktop(PathBuf),
    /// A command, for the sandboxed formats.
    Command(&'static str, Vec<String>),
}

fn in_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(program).is_file()))
}

/// The folders where the desktop looks for applications: the ones of the
/// user, of the system and of Flatpak.
fn application_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let mut dirs = vec![std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"))];
    let system = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    dirs.extend(system.split(':').filter(|d| !d.is_empty()).map(PathBuf::from));
    dirs.push(home.join(".local/share/flatpak/exports/share"));
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share"));
    dirs.into_iter().map(|d| d.join("applications")).collect()
}

/// The first desktop file with one of `names` that is not hidden.
fn find_desktop(names: &[String]) -> Option<PathBuf> {
    let dirs = application_dirs();
    for name in names.iter().filter(|n| !n.is_empty()) {
        let file = if name.ends_with(".desktop") { name.clone() } else { format!("{name}.desktop") };
        for dir in &dirs {
            let path = dir.join(&file);
            if let Ok(text) = std::fs::read_to_string(&path) {
                if !text.lines().any(|l| l.trim() == "Hidden=true") {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// How to start the app `app_id`, installed from `source` as `ref_id`; None
/// when it has nothing to start (a library, a tool of the terminal, firmware).
fn launch_for(app_id: &str, source: &str, ref_id: &str) -> Option<Launch> {
    match source {
        "flatpak" => Some(Launch::Command("flatpak", vec!["run".into(), ref_id.into()])),
        "snap" => Some(Launch::Command("snap", vec!["run".into(), ref_id.into()])),
        "firmware" => None,
        _ => {
            // The desktop files the catalog knows for the app, then its IDs.
            let mut names: Vec<String> = crate::apt::find(app_id, "").map(|a| a.desktop_ids.clone()).unwrap_or_default();
            names.extend([app_id.to_string(), ref_id.to_string()]);
            find_desktop(&names).map(Launch::Desktop)
        }
    }
}

/// Whether the installed app can be opened, so its page shows an Open button.
#[tauri::command(async)]
pub fn can_open_app(app_id: String, source: String, ref_id: String) -> bool {
    launch_for(&app_id, &source, &ref_id).is_some()
}

/// Opens an installed app, as its launcher in the menu does.
#[tauri::command(async)]
pub fn open_app(app_id: String, source: String, ref_id: String) -> Result<(), String> {
    start(launch_for(&app_id, &source, &ref_id).ok_or("The app has nothing to open")?)
}

/// The desktop files a Debian package installed and the menu shows: what an
/// installed .deb has to open, read from dpkg itself, so it works for any
/// package, also one that is in no catalog.
fn package_desktops(package: &str) -> Vec<PathBuf> {
    // Only a plain package name reaches dpkg.
    let valid = !package.is_empty()
        && !package.starts_with('-')
        && package.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if !valid {
        return Vec::new();
    }
    let Some(files) = system::output("dpkg-query", &["-L", package]) else { return Vec::new() };
    files
        .lines()
        .map(str::trim)
        .filter(|line| line.ends_with(".desktop") && line.contains("/applications/"))
        .map(PathBuf::from)
        .filter(|path| {
            std::fs::read_to_string(path)
                .map(|text| !text.lines().any(|l| matches!(l.trim(), "NoDisplay=true" | "Hidden=true")))
                .unwrap_or(false)
        })
        .collect()
}

/// Whether the package installed from a .deb file has something to open, so
/// its page shows an Open button once it is installed.
#[tauri::command(async)]
pub fn can_open_package(package: String) -> bool {
    !package_desktops(&package).is_empty()
}

/// Opens what the package installed from a .deb file put in the menu.
#[tauri::command(async)]
pub fn open_package(package: String) -> Result<(), String> {
    let desktop = package_desktops(&package).into_iter().next().ok_or("The package has nothing to open")?;
    start(Launch::Desktop(desktop))
}

/// Starts an application the way the desktop does.
fn start(launch: Launch) -> Result<(), String> {
    let (program, args): (String, Vec<String>) = match launch {
        Launch::Command(program, args) => (program.to_string(), args),
        Launch::Desktop(path) => {
            let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if in_path("gtk-launch") {
                ("gtk-launch".into(), vec![stem])
            } else if in_path("gio") {
                ("gio".into(), vec!["launch".into(), path.to_string_lossy().to_string()])
            } else if in_path("kioclient") {
                ("kioclient".into(), vec!["exec".into(), path.to_string_lossy().to_string()])
            } else {
                return Err("There is no program to start desktop files (gtk-launch, gio or kioclient)".into());
            }
        }
    };
    let mut child = Command::new(&program)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    // Waited for in the background, so it never stays as a zombie.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
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
