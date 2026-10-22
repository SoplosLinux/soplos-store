// Updates for the AppImages in ~/AppImages, with the same logic as
// Soplos AppImage Manager ("Search for updates"): the version and the
// address of the releases are written by hand in that app, which keeps them
// in the launcher it makes for each AppImage (`X-AppImage-Version` and
// `X-AppImage-UpdateUrl`). The Store only reads them. Two ways to look:
//   api    -> the address is a GitHub, GitLab or Codeberg project: the
//             latest release is read from that site's API and its version is
//             compared with the saved one.
//   direct -> the address ends in .AppImage: the date of the file on the
//             server is compared with the date of the installed one.
// An AppImage without a version or an address is skipped.
//
// Updating downloads the new file, replaces the old one and rewrites the
// launcher with the new version and path, which is what AppImage Manager
// does too, so it sees the AppImage as updated.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::appimage::{self, InstalledAppImage};
use crate::http;

const USER_AGENT: &str = "soplos-store";

pub struct Found {
    /// File name inside ~/AppImages.
    pub file: String,
    pub name: String,
    pub from: String,
    pub to: String,
    download_url: String,
    asset_name: String,
    /// Version to write in the launcher; empty for a direct link, which
    /// gives none.
    latest_version: String,
}

struct Asset {
    name: String,
    url: String,
}

fn parse_github(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("https://github.com/").or_else(|| url.strip_prefix("http://github.com/"))?;
    let mut parts = rest.splitn(2, '/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.split(['/', '?', '#']).next()?.trim_end_matches(".git").to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner, repo))
}

fn parse_gitlab(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("https://gitlab.com/").or_else(|| url.strip_prefix("http://gitlab.com/"))?;
    let mut parts = rest.splitn(2, '/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.split(['/', '?', '#']).next()?.trim_end_matches(".git").to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner, repo))
}

fn parse_forgejo(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("https://codeberg.org/").or_else(|| url.strip_prefix("http://codeberg.org/"))?;
    let mut parts = rest.splitn(2, '/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.split(['/', '?', '#']).next()?.trim_end_matches(".git").to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner, repo))
}

fn is_api_address(url: &str) -> bool {
    parse_github(url).is_some() || parse_gitlab(url).is_some() || parse_forgejo(url).is_some()
}

fn is_direct_address(url: &str) -> bool {
    url.to_lowercase().ends_with(".appimage")
}

/// Numbers of a version, to compare two of them ("v1.2.10" -> [1, 2, 10]).
fn version_numbers(version: &str) -> Vec<u32> {
    version.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect()
}

fn arch_aliases() -> &'static [&'static str] {
    match std::env::consts::ARCH {
        "aarch64" => &["aarch64", "arm64"],
        "x86" => &["i686", "x86"],
        _ => &["x86_64", "amd64"],
    }
}

fn json(url: &str, headers: &[(&str, &str)]) -> Option<Value> {
    serde_json::from_slice(&http::get(url, headers)?).ok()
}

/// AppImage assets listed with a name and an address (GitHub, Codeberg).
fn assets_named(list: Option<&Value>) -> Vec<Asset> {
    list.and_then(|v| v.as_array())
        .map(|assets| {
            assets
                .iter()
                .filter_map(|a| {
                    let name = a.get("name")?.as_str()?.to_string();
                    let url = a.get("browser_download_url")?.as_str()?.to_string();
                    name.to_lowercase().ends_with(".appimage").then_some(Asset { name, url })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn tag(release: &Value) -> String {
    release.get("tag_name").and_then(|v| v.as_str()).unwrap_or("").trim_start_matches('v').to_string()
}

/// Version and AppImage files of the latest release of the address.
fn latest_release(update_url: &str) -> Option<(String, Vec<Asset>)> {
    if let Some((owner, repo)) = parse_github(update_url) {
        let data = json(
            &format!("https://api.github.com/repos/{owner}/{repo}/releases/latest"),
            &[("User-Agent", USER_AGENT), ("Accept", "application/vnd.github+json")],
        )?;
        return Some((tag(&data), assets_named(data.get("assets"))));
    }
    if let Some((owner, repo)) = parse_forgejo(update_url) {
        let data = json(&format!("https://codeberg.org/api/v1/repos/{owner}/{repo}/releases?limit=1"), &[("User-Agent", USER_AGENT)])?;
        let release = data.as_array()?.first()?.clone();
        return Some((tag(&release), assets_named(release.get("assets"))));
    }
    if let Some((owner, repo)) = parse_gitlab(update_url) {
        let data = json(&format!("https://gitlab.com/api/v4/projects/{owner}%2F{repo}/releases"), &[("User-Agent", USER_AGENT)])?;
        let release = data.as_array()?.first()?.clone();
        let links = release.get("assets").and_then(|v| v.get("links")).and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let assets = links
            .iter()
            .filter_map(|link| {
                let name = link.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let url = link.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                (name.to_lowercase().contains(".appimage") || url.to_lowercase().contains(".appimage")).then_some(Asset { name, url })
            })
            .collect();
        return Some((tag(&release), assets));
    }
    None
}

fn check_api(app: &InstalledAppImage) -> Option<Found> {
    let (latest, assets) = latest_release(&app.update_url)?;
    if latest.is_empty() || assets.is_empty() {
        return None;
    }
    let current = app.version.trim_start_matches('v');
    let current_numbers = version_numbers(current);
    // A saved version with no number in it cannot be compared.
    if current_numbers.is_empty() || version_numbers(&latest) <= current_numbers {
        return None;
    }
    let aliases = arch_aliases();
    let best = assets.iter().find(|a| aliases.iter().any(|p| a.name.to_lowercase().contains(p))).unwrap_or(&assets[0]);
    Some(Found {
        file: app.file.clone(),
        name: app.name.clone(),
        from: app.version.clone(),
        to: latest.clone(),
        download_url: best.url.clone(),
        asset_name: best.name.clone(),
        latest_version: latest,
    })
}

fn check_direct(app: &InstalledAppImage) -> Option<Found> {
    let head = http::head(&app.update_url, &[("User-Agent", USER_AGENT)])?;
    // AppImage Manager offers the download when the server gives no date.
    // Here that would list the same update for ever, so it is skipped: an
    // update is shown only when the server's file is known to be newer.
    let remote = httpdate::parse_http_date(&head.last_modified?).ok()?;
    let local = fs::metadata(&app.path).and_then(|m| m.modified()).ok()?;
    if remote <= local {
        return None;
    }
    let mut asset_name = app.update_url.rsplit('/').next().unwrap_or("").to_string();
    if !asset_name.to_lowercase().ends_with(".appimage") {
        asset_name.push_str(".AppImage");
    }
    Some(Found {
        file: app.file.clone(),
        name: app.name.clone(),
        from: app.version.clone(),
        to: asset_name.clone(),
        download_url: app.update_url.clone(),
        asset_name,
        latest_version: String::new(),
    })
}

fn check(app: &InstalledAppImage) -> Option<Found> {
    if app.version.is_empty() || app.update_url.is_empty() {
        return None;
    }
    if is_api_address(&app.update_url) {
        check_api(app)
    } else if is_direct_address(&app.update_url) {
        check_direct(app)
    } else {
        None
    }
}

/// AppImages that can be looked at: they have a version and an address.
fn checkable() -> Vec<InstalledAppImage> {
    appimage::installed().into_iter().filter(|a| !a.version.is_empty() && !a.update_url.is_empty()).collect()
}

/// How many AppImages have no version or no address saved (and so are not
/// looked at); they are filled in by hand in AppImage Manager.
pub fn without_data() -> usize {
    appimage::installed().iter().filter(|a| a.version.is_empty() || a.update_url.is_empty()).count()
}

/// Whether there is any AppImage to look at.
pub fn any_checkable() -> bool {
    !checkable().is_empty()
}

/// The AppImages with a newer version. Each one is asked at the same time;
/// the requests have a time limit, so a server that does not answer cannot
/// hold the rest up.
pub fn updates() -> Vec<Found> {
    let apps = checkable();
    std::thread::scope(|scope| {
        let handles: Vec<_> = apps.iter().map(|app| scope.spawn(move || check(app))).collect();
        handles.into_iter().filter_map(|h| h.join().ok().flatten()).collect()
    })
}

/// Path for the new file: the name of the downloaded asset inside
/// ~/AppImages, with a number added when another file already has it.
fn new_path(dir: &Path, asset_name: &str, old: &Path) -> Result<PathBuf, String> {
    let mut name = Path::new(asset_name).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if name.is_empty() || name.starts_with('.') {
        return Err(format!("unusable file name: {asset_name}"));
    }
    if !name.to_lowercase().ends_with(".appimage") {
        name.push_str(".AppImage");
    }
    let mut path = dir.join(&name);
    if path.exists() && path != old {
        let file = Path::new(&name);
        let stem = file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let ext = file.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let mut i = 1;
        while path.exists() && path != old {
            path = dir.join(format!("{stem}_{i}.{ext}"));
            i += 1;
        }
    }
    Ok(path)
}

/// Writes the new path and version into the launcher, keeping everything
/// else in it: the name, the icon, the address, the root flag.
fn rewrite_launcher(desktop: &Path, path: &Path, version: &str) -> std::io::Result<()> {
    let content = fs::read_to_string(desktop)?;
    let path = path.display();
    let mut out = String::new();
    let mut saw_version = false;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("Exec=") {
            if value.trim_start().starts_with("pkexec ") {
                out.push_str(&format!("Exec=pkexec {path}\n"));
            } else {
                out.push_str(&format!("Exec={path}\n"));
            }
        } else if line.starts_with("TryExec=") {
            out.push_str(&format!("TryExec={path}\n"));
        } else if line.starts_with("X-AppImage-Version=") && !version.is_empty() {
            saw_version = true;
            out.push_str(&format!("X-AppImage-Version={version}\n"));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !version.is_empty() && !saw_version {
        out.push_str(&format!("X-AppImage-Version={version}\n"));
    }
    fs::write(desktop, out)
}

/// Downloads the newer version of one AppImage and puts it in place of the
/// old one. `say` receives the lines for the job log.
pub fn apply(file: &str, say: &mut dyn FnMut(String)) -> Result<(), String> {
    let app = appimage::installed().into_iter().find(|a| a.file == file).ok_or_else(|| format!("{file}: not found in ~/AppImages"))?;
    let found = check(&app).ok_or_else(|| format!("{}: no newer version found", app.name))?;
    let dir = appimage::appimages_dir()?;
    let new = new_path(&dir, &found.asset_name, &app.path)?;
    let part = new.with_extension("part");

    say(crate::i18n::t("checklog.appimageDownloading"));
    let mut last = 0;
    let result = http::download(&found.download_url, &part, &[("User-Agent", USER_AGENT)], |done, total| {
        if let Some(total) = total.filter(|t| *t > 0) {
            let percent = (done * 100 / total) / 10 * 10;
            if percent > last {
                last = percent;
                say(format!("{}  {percent}%", found.name));
            }
        }
    });
    if let Err(error) = result {
        let _ = fs::remove_file(&part);
        return Err(error);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&part, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    if app.path != new {
        let _ = fs::remove_file(&app.path);
    }
    fs::rename(&part, &new).map_err(|e| e.to_string())?;

    if let Some(desktop) = &app.desktop_path {
        rewrite_launcher(desktop, &new, &found.latest_version).map_err(|e| e.to_string())?;
    }
    appimage::update_desktop_db();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("soplos-store-test-{}-{name}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn app(version: &str, url: &str) -> InstalledAppImage {
        InstalledAppImage {
            file: "demo.AppImage".into(),
            name: "Demo".into(),
            summary: String::new(),
            version: version.into(),
            icon: None,
            size: String::new(),
            path: PathBuf::from("/nonexistent/demo.AppImage"),
            update_url: url.into(),
            desktop_path: None,
        }
    }

    #[test]
    fn versions_are_compared_by_their_numbers() {
        assert!(version_numbers("v1.2.10") > version_numbers("1.2.9"));
        assert_eq!(version_numbers("0.3.8"), vec![0, 3, 8]);
        assert!(version_numbers("beta").is_empty());
    }

    #[test]
    fn addresses_are_told_apart() {
        assert!(is_api_address("https://github.com/owner/repo"));
        assert!(is_api_address("https://github.com/owner/repo/releases/tag/v1"));
        assert!(is_api_address("https://gitlab.com/owner/repo"));
        assert!(is_api_address("https://codeberg.org/owner/repo"));
        assert!(!is_api_address("https://example.com/app.AppImage"));
        assert!(is_direct_address("https://example.com/app.AppImage"));
        assert!(!is_direct_address("https://example.com/page"));
        assert_eq!(parse_github("https://github.com/owner/repo.git"), Some(("owner".into(), "repo".into())));
    }

    #[test]
    fn appimages_without_version_or_address_are_skipped() {
        assert!(check(&app("", "https://github.com/owner/repo")).is_none());
        assert!(check(&app("1.0", "")).is_none());
        assert!(check(&app("1.0", "https://example.com/not-an-appimage")).is_none());
    }

    #[test]
    fn the_launcher_keeps_everything_but_the_path_and_the_version() {
        let dir = temp_dir("launcher");
        let desktop = dir.join("demo.desktop");
        fs::write(
            &desktop,
            "[Desktop Entry]\nType=Application\nName=Demo\nExec=pkexec /home/u/AppImages/Demo-1.0.AppImage\nTryExec=/home/u/AppImages/Demo-1.0.AppImage\nIcon=/home/u/AppImages/.icons/demo.png\nCategories=Utility;\nX-AppImage-Version=1.0\nX-AppImage-UpdateUrl=https://github.com/owner/repo\nX-AppImage-RunAsRoot=true\nX-AppImage-Integrate=true\n",
        )
        .unwrap();
        rewrite_launcher(&desktop, Path::new("/home/u/AppImages/Demo-1.1.AppImage"), "1.1").unwrap();
        let content = fs::read_to_string(&desktop).unwrap();
        assert!(content.contains("Exec=pkexec /home/u/AppImages/Demo-1.1.AppImage\n"));
        assert!(content.contains("TryExec=/home/u/AppImages/Demo-1.1.AppImage\n"));
        assert!(content.contains("X-AppImage-Version=1.1\n"));
        for kept in ["Name=Demo", "Icon=/home/u/AppImages/.icons/demo.png", "Categories=Utility;", "X-AppImage-UpdateUrl=https://github.com/owner/repo", "X-AppImage-RunAsRoot=true", "X-AppImage-Integrate=true"] {
            assert!(content.contains(kept), "lost: {kept}");
        }
        assert!(!content.contains("1.0"));
        // A direct link gives no version: the saved one stays as it was.
        rewrite_launcher(&desktop, Path::new("/home/u/AppImages/Demo-1.1.AppImage"), "").unwrap();
        assert!(fs::read_to_string(&desktop).unwrap().contains("X-AppImage-Version=1.1\n"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_new_file_never_replaces_another_one() {
        let dir = temp_dir("names");
        let old = dir.join("Demo-1.0.AppImage");
        fs::write(&old, b"old").unwrap();
        fs::write(dir.join("Demo-1.1.AppImage"), b"other").unwrap();
        assert_eq!(new_path(&dir, "Demo-1.1.AppImage", &old).unwrap(), dir.join("Demo-1.1_1.AppImage"));
        assert_eq!(new_path(&dir, "Demo-1.2.AppImage", &old).unwrap(), dir.join("Demo-1.2.AppImage"));
        assert_eq!(new_path(&dir, "Demo-1.0.AppImage", &old).unwrap(), old);
        assert!(new_path(&dir, ".hidden.AppImage", &old).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    // These two read the real GitHub API, so they only run on request:
    // cargo test -- --ignored
    #[test]
    #[ignore]
    fn a_real_release_is_found_when_the_saved_version_is_old() {
        let found = check(&app("0.1.0", "https://github.com/domsvoboda/xodus-nexus")).expect("a newer release");
        assert!(found.download_url.to_lowercase().ends_with(".appimage"), "{}", found.download_url);
        println!("{} -> {} ({})", found.from, found.to, found.asset_name);
    }

    #[test]
    #[ignore]
    fn nothing_is_offered_when_the_saved_version_is_newer() {
        assert!(check(&app("99.0.0", "https://github.com/domsvoboda/xodus-nexus")).is_none());
    }

    // Downloads a real AppImage (about 130 MB) into a temporary HOME:
    // cargo test -- --ignored --test-threads=1
    #[test]
    #[ignore]
    fn a_full_update_leaves_a_launcher_that_appimage_manager_reads() {
        let home = temp_dir("home");
        std::env::set_var("HOME", &home);
        let dir = home.join("AppImages");
        let applications = home.join(".local/share/applications");
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir_all(&applications).unwrap();
        let old = dir.join("Xodus-Nexus-0.3.0-x86_64.AppImage");
        fs::write(&old, b"not a real appimage").unwrap();
        let desktop = applications.join("soplos-appimage-xodus-nexus.desktop");
        fs::write(
            &desktop,
            format!(
                "[Desktop Entry]\nType=Application\nName=Xodus Nexus\nExec={0}\nTryExec={0}\nIcon=application-x-executable\nCategories=Game;\nX-AppImage-Version=0.3.0\nX-AppImage-UpdateUrl=https://github.com/domsvoboda/xodus-nexus\nX-AppImage-Integrate=true\n",
                old.display()
            ),
        )
        .unwrap();

        let found = updates();
        assert_eq!(found.len(), 1, "one update expected");
        println!("update: {} -> {}", found[0].from, found[0].to);

        let mut lines = Vec::new();
        apply("Xodus-Nexus-0.3.0-x86_64.AppImage", &mut |line| lines.push(line)).unwrap();
        println!("{}", lines.join("\n"));

        // The old file is gone and the new one is in place and executable.
        assert!(!old.exists());
        let listed = appimage::installed();
        assert_eq!(listed.len(), 1);
        let now = &listed[0];
        assert_ne!(now.version, "0.3.0");
        assert!(now.path.is_file() && now.path != old);
        let mut magic = [0u8; 4];
        std::io::Read::read_exact(&mut fs::File::open(&now.path).unwrap(), &mut magic).unwrap();
        assert_eq!(&magic, b"\x7fELF");
        // The launcher kept its address and name, and points to the new file.
        let launcher = fs::read_to_string(&desktop).unwrap();
        assert!(launcher.contains(&format!("Exec={}\n", now.path.display())));
        assert!(launcher.contains("X-AppImage-UpdateUrl=https://github.com/domsvoboda/xodus-nexus"));
        assert!(launcher.contains("Name=Xodus Nexus"));
        assert_eq!(now.name, "Xodus Nexus");
        // And it is up to date now.
        assert!(updates().is_empty());
        let _ = fs::remove_dir_all(&home);
    }
}
