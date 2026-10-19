// AppImage management: 1:1 match with soplos-appimage-manager-3.0
// Manages AppImages in ~/AppImages, launchers in ~/.local/share/applications,
// and extracted icons in ~/AppImages/.icons/.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::system;

pub struct InstalledAppImage {
    pub file: String,
    pub name: String,
    pub summary: String,
    pub version: String,
    pub icon: Option<String>,
    pub size: String,
}

pub fn home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME is not set".to_string())
}

pub fn appimages_dir() -> Result<PathBuf, String> {
    let dir = home_dir()?.join("AppImages");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn icons_dir() -> Result<PathBuf, String> {
    let dir = appimages_dir()?.join(".icons");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn desktop_dir() -> Result<PathBuf, String> {
    let dir = home_dir()?.join(".local/share/applications");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn is_appimage_file(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".appimage") && !name.starts_with('.')
}

fn safe_id(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

fn run_with_timeout(cmd: &mut Command, timeout: Duration) -> bool {
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let Ok(mut child) = cmd.spawn() else {
        return false;
    };
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => return true,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return false;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return false,
        }
    }
}

pub struct ExtractedMeta {
    pub name: Option<String>,
    pub icon_path: Option<PathBuf>,
    pub version: String,
    pub comment: String,
    pub categories: String,
}

fn which(bin: &str) -> bool {
    std::env::var("PATH")
        .ok()
        .map(|path| path.split(':').any(|dir| Path::new(dir).join(bin).is_file()))
        .unwrap_or(false)
}

fn extract_appimage(appimage_path: &Path, dest_dir: &Path) -> Option<PathBuf> {
    let squashfs_root = dest_dir.join("squashfs-root");

    let sevenzip = if which("7zz") {
        Some("7zz")
    } else if which("7z") {
        Some("7z")
    } else {
        None
    };

    if let Some(tool) = sevenzip {
        let mut cmd = Command::new(tool);
        cmd.args([
            "x",
            &appimage_path.to_string_lossy(),
            &format!("-o{}", squashfs_root.to_string_lossy()),
            "-y",
            "-bso0",
            "-bsp0",
            "-ir!*.png",
            "-ir!*.svg",
            "-ir!*.desktop",
            "-ir0!.DirIcon",
        ])
        .current_dir(dest_dir);
        let _ = run_with_timeout(&mut cmd, Duration::from_secs(60));
        if squashfs_root.exists() {
            return Some(squashfs_root);
        }
    }

    // Fallback: copy the AppImage into the work dir and run --appimage-extract
    let tmp_copy = dest_dir.join("app.appimage");
    if fs::copy(appimage_path, &tmp_copy).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&tmp_copy, fs::Permissions::from_mode(0o755));
        }
        let mut cmd = Command::new(&tmp_copy);
        cmd.arg("--appimage-extract").current_dir(dest_dir);
        let _ = run_with_timeout(&mut cmd, Duration::from_secs(120));
        if squashfs_root.exists() {
            return Some(squashfs_root);
        }
    }

    None
}

fn find_icon(root: &Path, icon_name: &str) -> Option<PathBuf> {
    // 1. .DirIcon (file or symlink)
    let diricon = root.join(".DirIcon");
    if let Ok(link_target) = fs::read_link(&diricon) {
        let real = if link_target.is_absolute() {
            link_target
        } else {
            diricon.parent().unwrap_or(root).join(link_target)
        };
        if real.is_file() {
            return Some(real);
        }
    } else if diricon.is_file() {
        return Some(diricon);
    }

    let name_lower = icon_name.to_lowercase();

    // 2. Root-level {name}.svg / {name}.png
    for ext in ["svg", "png"] {
        let candidate = root.join(format!("{icon_name}.{ext}"));
        if candidate.exists() {
            return Some(candidate);
        }
    }
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext_ok = path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("svg") || e.eq_ignore_ascii_case("png")).unwrap_or(false);
            if ext_ok && path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_lowercase() == name_lower).unwrap_or(false) {
                return Some(path);
            }
        }
    }

    // 3. hicolor icon theme dirs
    let hicolor = root.join("usr/share/icons/hicolor");
    for size in ["scalable/apps", "512x512/apps", "256x256/apps", "128x128/apps", "64x64/apps", "48x48/apps"] {
        for ext in ["png", "svg"] {
            let candidate = hicolor.join(size).join(format!("{icon_name}.{ext}"));
            if candidate.exists() {
                return Some(candidate);
            }
            let size_dir = hicolor.join(size);
            if let Ok(entries) = fs::read_dir(&size_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case(ext)).unwrap_or(false)
                        && path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_lowercase() == name_lower).unwrap_or(false)
                    {
                        return Some(path);
                    }
                }
            }
        }
    }

    // 4. usr/share/pixmaps/
    let pixmaps = root.join("usr/share/pixmaps");
    if pixmaps.is_dir() {
        for ext in ["png", "svg"] {
            let candidate = pixmaps.join(format!("{icon_name}.{ext}"));
            if candidate.exists() {
                return Some(candidate);
            }
        }
        if let Ok(entries) = fs::read_dir(&pixmaps) {
            for entry in entries.flatten() {
                let path = entry.path();
                let ext_ok = path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("svg") || e.eq_ignore_ascii_case("png")).unwrap_or(false);
                if ext_ok && path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_lowercase() == name_lower).unwrap_or(false) {
                    return Some(path);
                }
            }
        }
    }

    // 5. Any .png then .svg anywhere under root
    for ext in ["png", "svg"] {
        let mut candidates = Vec::new();
        collect_by_ext(root, ext, &mut candidates);
        if let Some(exact) = candidates.iter().find(|c| c.file_stem().and_then(|s| s.to_str()).map(|s| s.to_lowercase() == name_lower).unwrap_or(false)) {
            return Some(exact.clone());
        }
        if let Some(first) = candidates.into_iter().next() {
            return Some(first);
        }
    }

    None
}

fn collect_by_ext(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_by_ext(&path, ext, out);
        } else if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case(ext)).unwrap_or(false) {
            out.push(path);
        }
    }
}

pub fn parse_desktop_file(path: &Path) -> HashMap<String, String> {
    let mut entry = HashMap::new();
    let Ok(content) = fs::read_to_string(path) else { return entry };
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[Desktop Entry]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') {
            if in_section {
                break;
            }
            continue;
        }
        if !in_section || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            entry.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    entry
}

pub fn extract_metadata(appimage_path: &Path, app_id: &str, icons_dir: &Path) -> ExtractedMeta {
    let mut result = ExtractedMeta {
        name: None,
        icon_path: None,
        version: String::new(),
        comment: String::new(),
        categories: "Utility;".to_string(),
    };

    let tmp_dir = std::env::temp_dir().join(format!("soplos-store-extract-{app_id}-{}", std::process::id()));
    let _ = fs::create_dir_all(&tmp_dir);

    if let Some(root) = extract_appimage(appimage_path, &tmp_dir) {
        let desktop_file = fs::read_dir(&root)
            .ok()
            .and_then(|entries| entries.flatten().find(|e| e.path().extension().and_then(|x| x.to_str()) == Some("desktop")))
            .map(|e| e.path())
            .or_else(|| {
                let mut found = None;
                find_desktop_recursive(&root, &mut found);
                found
            });

        if let Some(desktop_path) = desktop_file {
            let entry = parse_desktop_file(&desktop_path);
            if let Some(name) = entry.get("Name") {
                result.name = Some(name.clone());
            }
            result.version = entry.get("X-AppImage-Version").cloned().unwrap_or_default();
            result.comment = entry.get("Comment").cloned().unwrap_or_default();
            result.categories = entry.get("Categories").cloned().unwrap_or_else(|| "Utility;".to_string());

            let icon_name = entry.get("Icon").cloned().unwrap_or_default();
            let icon_name = icon_name.trim_end_matches(".png").trim_end_matches(".svg");
            if !icon_name.is_empty() {
                if let Some(icon_src) = find_icon(&root, icon_name) {
                    let suffix = icon_src.extension().and_then(|e| e.to_str()).unwrap_or("png");
                    let icon_dest = icons_dir.join(format!("{app_id}.{suffix}"));
                    if fs::copy(&icon_src, &icon_dest).is_ok() {
                        result.icon_path = Some(icon_dest);
                    }
                }
            }
        }
    }

    let _ = fs::remove_dir_all(&tmp_dir);
    result
}

fn find_desktop_recursive(dir: &Path, found: &mut Option<PathBuf>) {
    if found.is_some() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_desktop_recursive(&path, found);
        } else if path.extension().and_then(|e| e.to_str()) == Some("desktop") {
            *found = Some(path);
            return;
        }
        if found.is_some() {
            return;
        }
    }
}

#[allow(dead_code)]
pub fn detect_appimage_type(path: &Path) -> String {
    let mut buf = [0u8; 11];
    let Ok(mut file) = fs::File::open(path) else {
        return "?".to_string();
    };
    if file.read_exact(&mut buf).is_err() {
        return "?".to_string();
    }
    let magic = &buf[8..11];
    if magic == [0x41, 0x49, 0x01] {
        "1".to_string()
    } else if magic == [0x41, 0x49, 0x02] {
        "2".to_string()
    } else {
        "?".to_string()
    }
}

fn unique_desktop_path(app_id: &str) -> Result<PathBuf, String> {
    let dir = desktop_dir()?;
    let mut path = dir.join(format!("soplos-appimage-{app_id}.desktop"));
    let mut i = 1;
    while path.exists() {
        path = dir.join(format!("soplos-appimage-{app_id}_{i}.desktop"));
        i += 1;
    }
    Ok(path)
}

fn unique_dest_path(dir: &Path, source: &Path) -> PathBuf {
    let mut dest = dir.join(source.file_name().unwrap_or_default());
    if dest == *source {
        return dest;
    }
    let stem = source.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = source.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let mut i = 1;
    while dest.exists() {
        dest = dir.join(format!("{stem}_{i}.{ext}"));
        i += 1;
    }
    dest
}

fn update_desktop_db() {
    if let Ok(dir) = desktop_dir() {
        let _ = Command::new("update-desktop-database").arg(dir).arg("-q").output();
    }
}

fn write_desktop_file(
    desktop_path: &Path,
    name: &str,
    exec_path: &Path,
    icon: &str,
    comment: &str,
    categories: &str,
    version: &str,
    update_url: &str,
    run_as_root: bool,
) -> std::io::Result<()> {
    let exec_value = if run_as_root {
        format!("pkexec {}", exec_path.display())
    } else {
        exec_path.display().to_string()
    };
    let mut content = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec={exec_value}\nTryExec={}\nIcon={icon}\nCategories={categories}\n",
        exec_path.display()
    );
    if !comment.is_empty() {
        content.push_str(&format!("Comment={comment}\n"));
    }
    if !version.is_empty() {
        content.push_str(&format!("X-AppImage-Version={version}\n"));
    }
    if !update_url.is_empty() {
        content.push_str(&format!("X-AppImage-UpdateUrl={update_url}\n"));
    }
    if run_as_root {
        content.push_str("X-AppImage-RunAsRoot=true\n");
    }
    content.push_str("X-AppImage-Integrate=true\n");
    fs::write(desktop_path, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(desktop_path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

fn integrate(dest_path: PathBuf) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dest_path, fs::Permissions::from_mode(0o755));
    }

    let app_id = safe_id(&dest_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
    let icons_dir = icons_dir()?;
    let meta = extract_metadata(&dest_path, &app_id, &icons_dir);

    let name = meta.name.unwrap_or_else(|| dest_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
    let final_id = safe_id(&name);
    let icon_value = meta.icon_path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "application-x-executable".to_string());

    let desktop_path = unique_desktop_path(&final_id)?;
    write_desktop_file(
        &desktop_path,
        &name,
        &dest_path,
        &icon_value,
        &meta.comment,
        &meta.categories,
        &meta.version,
        "",
        false,
    )
    .map_err(|e| e.to_string())?;
    update_desktop_db();

    Ok(())
}

/// Lists installed AppImages from ~/AppImages, exactly matching AppImage Manager's list logic.
pub fn installed() -> Vec<InstalledAppImage> {
    let Ok(appimages_dir) = appimages_dir() else {
        return Vec::new();
    };
    let Ok(desktop_dir) = desktop_dir() else {
        return Vec::new();
    };

    let mut desktop_map: HashMap<String, (PathBuf, HashMap<String, String>)> = HashMap::new();
    if let Ok(entries) = fs::read_dir(&desktop_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let parsed = parse_desktop_file(&path);
            let exec_val = parsed
                .get("TryExec")
                .cloned()
                .or_else(|| parsed.get("Exec").and_then(|e| e.split_whitespace().next()).map(|s| s.trim_start_matches("pkexec ").to_string()));
            if let Some(exec_val) = exec_val {
                if exec_val.starts_with(&appimages_dir.to_string_lossy().to_string()) {
                    desktop_map.insert(exec_val, (path, parsed));
                }
            }
        }
    }

    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir(&appimages_dir) else {
        return result;
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_appimage_file(&p.file_name().unwrap_or_default().to_string_lossy()))
        .collect();
    paths.sort();

    for path in paths {
        let key = path.to_string_lossy().to_string();
        let match_entry = desktop_map.get(&key);

        let mut name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let mut version = String::new();
        let mut comment = String::new();
        let mut icon_uri = None;

        if let Some((_desktop_path, entry)) = match_entry {
            if let Some(n) = entry.get("Name") {
                name = n.clone();
            }
            version = entry.get("X-AppImage-Version").cloned().unwrap_or_default();
            comment = entry.get("Comment").cloned().unwrap_or_default();
            if let Some(icon_raw) = entry.get("Icon") {
                let p = Path::new(icon_raw);
                if p.is_absolute() && p.is_file() {
                    icon_uri = system::file_data_uri(p);
                } else if !icon_raw.is_empty() {
                    // Check in ~/AppImages/.icons/
                    let in_icons_png = appimages_dir.join(".icons").join(format!("{icon_raw}.png"));
                    let in_icons_svg = appimages_dir.join(".icons").join(format!("{icon_raw}.svg"));
                    if in_icons_png.is_file() {
                        icon_uri = system::file_data_uri(&in_icons_png);
                    } else if in_icons_svg.is_file() {
                        icon_uri = system::file_data_uri(&in_icons_svg);
                    } else {
                        icon_uri = crate::desktop::resolve_icon(icon_raw);
                    }
                }
            }
        }

        let file = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let meta_len = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        result.push(InstalledAppImage {
            file,
            name,
            summary: comment,
            version,
            icon: icon_uri,
            size: system::format_size(meta_len),
        });
    }

    result
}

/// Remove an AppImage file, its desktop launcher, and its icon from ~/AppImages/.icons/.
pub fn remove(file_name: &str) -> Result<(), String> {
    let dir = appimages_dir()?;
    let path = dir.join(file_name);
    if path.is_file() {
        let _ = fs::remove_file(&path);
    }

    if let Ok(desktop) = desktop_dir() {
        if let Ok(entries) = fs::read_dir(&desktop) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|x| x.to_str()) == Some("desktop") {
                    if let Ok(content) = fs::read_to_string(&p) {
                        if content.contains(file_name) {
                            let _ = fs::remove_file(&p);
                        }
                    }
                }
            }
        }
    }

    if let Ok(icons) = icons_dir() {
        let stem = file_name.rsplit_once('.').map(|(s, _)| s).unwrap_or(file_name);
        let id = safe_id(stem);
        for ext in ["png", "svg"] {
            let icon_file = icons.join(format!("{id}.{ext}"));
            if icon_file.is_file() {
                let _ = fs::remove_file(&icon_file);
            }
        }
    }

    update_desktop_db();
    Ok(())
}

/// Install/integrate an AppImage file into ~/AppImages, exactly like soplos-appimage-manager.
pub fn install_file(source_path: &Path) -> Result<(), String> {
    let dest_dir = appimages_dir()?;

    if let Some(file_name) = source_path.file_name() {
        let existing = dest_dir.join(file_name);
        if existing.exists() && existing != source_path {
            if let (Ok(existing_meta), Ok(source_meta)) = (fs::metadata(&existing), fs::metadata(source_path)) {
                if existing_meta.len() == source_meta.len() {
                    return Ok(()); // Already installed
                }
            }
        }
    }

    let dest = unique_dest_path(&dest_dir, source_path);

    if dest != source_path {
        fs::copy(source_path, &dest).map_err(|e| e.to_string())?;
    }

    integrate(dest)
}
