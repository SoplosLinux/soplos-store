// Install, remove and update jobs. Each job is a short list of commands
// run one after another; their output is sent to the window line by line
// ("job-line") and the result at the end ("job-done"). Only one job runs
// at a time, because apt cannot run twice at once.

use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::{appimage_update, apt, catalog, checker, flatpak, fwupd, i18n, nvidia, settings, validate, vendor};

const FLATHUB_REPO: &str = "https://dl.flathub.org/repo/flathub.flatpakrepo";

/// pkexec exit codes for a dismissed or refused password dialog.
const PKEXEC_CANCELLED: [i32; 2] = [126, 127];

/// `timeout` exit codes: it stopped the command (124) or had to kill it (137).
const TIMED_OUT: [i32; 2] = [124, 137];

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRequest {
    /// install, remove, update, install-file, refresh, autoremove, clean or unused.
    pub action: String,
    /// apt, flatpak, snap or firmware.
    pub source: String,
    #[serde(default)]
    pub ids: Vec<String>,
    /// Flatpak only: "user" or "system".
    pub scope: Option<String>,
    /// install-file only: path of the package file.
    pub file: Option<String>,
}

#[derive(Serialize, Clone)]
struct JobLine {
    id: u64,
    line: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct JobDone {
    id: u64,
    success: bool,
    cancelled: bool,
}

#[tauri::command]
pub fn start_job(app: AppHandle, request: JobRequest) -> Result<u64, String> {
    let steps = plan(&request)?;
    let is_check = request.action == "check";
    let refreshes_lists = (request.source == "apt" && request.action == "refresh") || is_check;
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    std::thread::spawn(move || {
        let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let mut success = true;
        let mut cancelled = false;
        let mut total = 0;
        for step in &steps {
            // Two markers in front of a command: "?" lets the job go on if
            // it fails, "#echo" only prints a line (a heading in the log).
            let (tolerant, command) = match step.split_first() {
                Some((first, rest)) if first == "?" => (true, rest),
                _ => (false, &step[..]),
            };
            if command[0] == "#echo" {
                let _ = app.emit("job-line", JobLine { id, line: command[1].clone() });
                continue;
            }
            if command[0] == "#scan" {
                total = scan_step(&app, id);
                continue;
            }
            if command[0] == "#appimage-update" {
                let mut say = |line: String| {
                    let _ = app.emit("job-line", JobLine { id, line });
                };
                if let Err(error) = appimage_update::apply(&command[1], &mut say) {
                    say(error);
                    success = false;
                    break;
                }
                continue;
            }
            match run_step(&app, id, command) {
                Ok(0) => {}
                Ok(code) => {
                    // `timeout` ends with 124 when it stopped the command, or
                    // 137 when it had to kill it; say so, because the command
                    // itself writes nothing when it is cut off.
                    if command[0] == "timeout" && TIMED_OUT.contains(&code) {
                        let _ = app.emit("job-line", JobLine { id, line: i18n::t("checklog.timedOut") });
                    }
                    if !tolerant {
                        success = false;
                        cancelled = command[0] == "pkexec" && PKEXEC_CANCELLED.contains(&code);
                        break;
                    }
                }
                Err(error) => {
                    let _ = app.emit("job-line", JobLine { id, line: error });
                    if !tolerant {
                        success = false;
                        break;
                    }
                }
            }
        }
        apt::invalidate_installed();
        if success && refreshes_lists {
            apt::mark_refreshed();
        }
        // Closing line of a check: the total, so the log ends on a result.
        if success && is_check {
            let line = if total == 0 { i18n::t("checklog.doneNone") } else { i18n::t_n("checklog.doneSome", total) };
            let _ = app.emit("job-line", JobLine { id, line });
        }
        let _ = app.emit("job-done", JobDone { id, success, cancelled });
    });
    Ok(id)
}

fn privileged(action: &str, args: &[String]) -> Result<Vec<String>, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut command = vec!["pkexec".to_string(), exe.to_string_lossy().into_owned(), "--privileged".into()];
    // pkexec starts the helper with a clean environment, so the language
    // of the app (and the locale that has its translations) go as options.
    command.push(format!("--lang={}", i18n::current()));
    if let Ok(locale) = std::env::var("LANG") {
        if locale.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '@' | '-')) && !locale.is_empty() {
            command.push(format!("--locale={locale}"));
        }
    }
    command.push(action.into());
    command.extend(args.iter().cloned());
    Ok(command)
}

fn privileged_refresh() -> Result<Vec<String>, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut command = vec!["pkexec".to_string(), exe.to_string_lossy().into_owned(), "--privileged-refresh".into()];
    command.push(format!("--lang={}", i18n::current()));
    if let Ok(locale) = std::env::var("LANG") {
        if locale.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '@' | '-')) && !locale.is_empty() {
            command.push(format!("--locale={locale}"));
        }
    }
    Ok(command)
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn scope_flag(request: &JobRequest) -> Result<String, String> {
    match request.scope.as_deref() {
        Some("user") | None => Ok("--user".into()),
        Some("system") => Ok("--system".into()),
        Some(other) => Err(format!("unknown Flatpak scope: {other}")),
    }
}

/// Adds the Flathub remote to the chosen installation when it is missing.
fn flathub_step(scope: &str) -> Option<Vec<String>> {
    (!flatpak::has_flathub(scope)).then(|| strings(&["flatpak", "remote-add", scope, "--if-not-exists", "flathub", FLATHUB_REPO]))
}

/// Turns a request into the commands to run, refusing anything invalid.
fn plan(request: &JobRequest) -> Result<Vec<Vec<String>>, String> {
    let ids = &request.ids;
    let file = request.file.clone().unwrap_or_default();
    let need_ids = |check: fn(&str) -> bool| -> Result<(), String> {
        if ids.is_empty() || !ids.iter().all(|id| check(id)) {
            Err("invalid package names".into())
        } else {
            Ok(())
        }
    };
    let steps = match (request.source.as_str(), request.action.as_str()) {
        ("apt", "install") => {
            need_ids(validate::apt_package)?;
            vec![privileged("apt-install", ids)?]
        }
        ("apt", "remove") => {
            need_ids(validate::apt_package)?;
            vec![privileged("apt-remove", ids)?]
        }
        ("apt", "update") => {
            need_ids(validate::apt_package)?;
            vec![privileged("apt-upgrade", ids)?]
        }
        ("apt", "install-file") if validate::package_file(&file, &["deb"]) => vec![privileged("apt-install-file", &[file])?],
        // Drivers: the branch or package is checked here against the closed
        // lists of nvidia.rs, and the helper that runs as root checks it again.
        ("driver", "install") => {
            let [id] = ids.as_slice() else {
                return Err("one driver at a time".into());
            };
            if nvidia::CUDA_BRANCHES.contains(&id.as_str()) {
                vec![privileged("nvidia-install-cuda", ids)?]
            } else if nvidia::REPO_PACKAGES.contains(&id.as_str()) {
                vec![privileged("nvidia-install-repo", ids)?]
            } else if vendor::is_item(id) {
                vec![privileged("vendor-install", ids)?]
            } else {
                return Err(format!("unknown driver: {id}"));
            }
        }
        ("driver", "remove") => vec![privileged("nvidia-remove", &[])?],
        ("apt", "refresh") => vec![privileged_refresh()?],
        ("apt", "autoremove") => vec![privileged("apt-autoremove", &[])?],
        ("apt", "clean") => vec![privileged("apt-clean", &[])?],

        ("flatpak", "install") => {
            need_ids(validate::flatpak_id)?;
            let scope = scope_flag(request)?;
            let mut install = strings(&["flatpak", "install", &scope, "-y", "--noninteractive", "flathub"]);
            install.extend(ids.iter().cloned());
            flathub_step(&scope).into_iter().chain(std::iter::once(install)).collect()
        }
        ("flatpak", "remove") => {
            need_ids(validate::flatpak_id)?;
            let installed = flatpak::installed();
            ids.iter()
                .map(|id| {
                    // Removed from whichever installation it is in.
                    let scope = installed.iter().find(|a| &a.id == id).map(|a| a.scope.as_str()).unwrap_or("user");
                    strings(&["flatpak", "uninstall", &format!("--{scope}"), "-y", "--noninteractive", id])
                })
                .collect()
        }
        ("flatpak", "update") => {
            need_ids(validate::flatpak_id)?;
            // Each app is updated in the installation it lives in; runtimes
            // and anything not found among the apps are left to flatpak.
            let installed = flatpak::installed();
            let mut groups: Vec<(&str, Vec<String>)> = vec![("--user", Vec::new()), ("--system", Vec::new()), ("", Vec::new())];
            for id in ids {
                let slot = match installed.iter().find(|a| &a.id == id).map(|a| a.scope.as_str()) {
                    Some("user") => 0,
                    Some("system") => 1,
                    _ => 2,
                };
                groups[slot].1.push(id.clone());
            }
            groups
                .into_iter()
                .filter(|(_, group)| !group.is_empty())
                .map(|(flag, group)| {
                    let mut update = strings(&["flatpak", "update", "-y", "--noninteractive"]);
                    if !flag.is_empty() {
                        update.push(flag.to_string());
                    }
                    update.extend(group);
                    update
                })
                .collect()
        }
        ("flatpak", "install-file") if validate::package_file(&file, &["flatpak", "flatpakref"]) => {
            let scope = scope_flag(request)?;
            let kind = if file.to_lowercase().ends_with(".flatpakref") { "--from" } else { "--bundle" };
            // Bundles take their runtime from Flathub.
            let install = strings(&["flatpak", "install", &scope, "-y", "--noninteractive", kind, &file]);
            flathub_step(&scope).into_iter().chain(std::iter::once(install)).collect()
        }
        ("flatpak", "unused") => vec![
            strings(&["flatpak", "uninstall", "--user", "--unused", "-y", "--noninteractive"]),
            strings(&["flatpak", "uninstall", "--system", "--unused", "-y", "--noninteractive"]),
        ],

        ("snap", "install") => {
            need_ids(validate::snap_name)?;
            ids.iter().map(|id| strings(&["snap", "install", id])).collect()
        }
        ("snap", "remove") => {
            need_ids(validate::snap_name)?;
            ids.iter().map(|id| strings(&["snap", "remove", id])).collect()
        }
        ("snap", "update") => {
            need_ids(validate::snap_name)?;
            let mut refresh = strings(&["snap", "refresh"]);
            refresh.extend(ids.iter().cloned());
            vec![refresh]
        }
        // A local .snap file is not signed by the store, hence --dangerous.
        ("snap", "install-file") if validate::package_file(&file, &["snap"]) => vec![strings(&["snap", "install", "--dangerous", &file])],

        ("appimage", "remove") => {
            for id in ids {
                let _ = crate::appimage::remove(id);
            }
            vec![vec!["#echo".to_string(), "OK".to_string()]]
        }
        // The download runs inside the job, so the plan only names the files.
        ("appimage", "update") => {
            need_ids(validate::appimage_file)?;
            ids.iter().map(|id| strings(&["#appimage-update", id])).collect()
        }
        ("appimage", "install-file") => {
            crate::appimage::install_file(std::path::Path::new(&file))?;
            vec![vec!["#echo".to_string(), "OK".to_string()]]
        }

        // Firmware goes through fwupd, which asks for authorization itself.
        ("firmware", "update") => {
            need_ids(validate::fwupd_device_id)?;
            // The limit is long because the download and the writing to the
            // device can take minutes; it is only there so an unreachable
            // LVFS cannot hold the job for good.
            ids.iter().map(|id| strings(&["timeout", "--kill-after=30", "1800", "fwupdmgr", "update", id, "-y", "--no-reboot-check"])).collect()
        }
        ("firmware", "testing-on") => vec![strings(&["fwupdmgr", "enable-remote", "lvfs-testing", "-y"]), firmware_refresh(true)],
        ("firmware", "testing-off") => vec![strings(&["fwupdmgr", "disable-remote", "lvfs-testing", "-y"])],

        // "Check now", in three steps: refresh apt's package lists (asks for
        // the password), refresh the firmware metadata, and look in every
        // source once ("#scan"). The scan stores its result, which is what
        // the Updates page and the tray notifier read when the job ends, so
        // nothing is asked a second time.
        ("all", "check") => {
            let heading = |key: &str| vec!["#echo".to_string(), i18n::t(key)];
            let mut steps: Vec<Vec<String>> = vec![heading("checklog.apt"), privileged_refresh()?];
            // Firmware is left out entirely when it is turned off in Settings.
            if fwupd::available() && settings::firmware_enabled() {
                steps.push(heading("checklog.firmware"));
                let mut refresh = firmware_refresh(false);
                refresh.insert(0, "?".into());
                steps.push(refresh);
            }
            steps.push(strings(&["#scan"]));
            steps
        }

        (source, action) => return Err(format!("unsupported job: {action} from {source}")),
    };
    Ok(steps)
}

/// Refreshes the LVFS firmware metadata. It goes through `timeout` because
/// fwupdmgr waits as long as the server takes to answer, and when the LVFS
/// is down that would hold up the whole job for good.
///
/// With `force` the metadata is downloaded again even when it is up to date;
/// the check does not need that, only turning on the testing channel does.
fn firmware_refresh(force: bool) -> Vec<String> {
    let mut command = strings(&["timeout", "--kill-after=10", "120", "fwupdmgr", "refresh"]);
    if force {
        command.push("--force".into());
    }
    command
}

/// The check itself: every source is asked once, all at the same time. The
/// log gets a block per source (the updates it found, then the count and how
/// long the source took to answer) and the result is stored for the page and
/// the tray notifier. Returns how many updates there are.
fn scan_step(app: &AppHandle, id: u64) -> usize {
    let say = |line: String| {
        let _ = app.emit("job-line", JobLine { id, line });
    };
    apt::invalidate_installed();
    let mut all = Vec::new();
    let (mut added, mut removed) = (Vec::new(), Vec::new());
    for source in catalog::scan() {
        for item in &source.items {
            say(format!("  {}  {} -> {}", item.name, item.from, item.to));
        }
        let result = if source.items.is_empty() { i18n::t("checklog.none") } else { i18n::t_n("checklog.found", source.items.len()) };
        say(format!("{}: {result} ({:.1} s)", source.label, source.took.as_secs_f32()));
        all.extend(source.items);
        added.extend(source.added);
        removed.extend(source.removed);
    }
    // AppImages without a version or an address are not looked at; they
    // are filled in by hand in AppImage Manager.
    let skipped = appimage_update::without_data();
    if skipped > 0 {
        say(i18n::t_n("checklog.appimageHint", skipped));
    }
    let total = all.len();
    checker::save_with_plan(all, added, removed);
    total
}

fn run_step(app: &AppHandle, id: u64, command: &[String]) -> Result<i32, String> {
    let mut child = Command::new(&command[0])
        .args(&command[1..])
        // Output in the language of the app (when the system has its
        // translations), English otherwise.
        .env("LANGUAGE", format!("{}:en", i18n::current()))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{}: {e}", command[0]))?;

    let stderr = child.stderr.take();
    let stderr_app = app.clone();
    let stderr_reader = std::thread::spawn(move || {
        if let Some(stream) = stderr {
            forward(&stderr_app, id, stream);
        }
    });
    if let Some(stream) = child.stdout.take() {
        forward(app, id, stream);
    }
    let _ = stderr_reader.join();
    let status = child.wait().map_err(|e| e.to_string())?;
    Ok(status.code().unwrap_or(1))
}

/// Sends a stream to the window line by line; apt and flatpak redraw
/// their progress with carriage returns, so those split lines too.
fn forward(app: &AppHandle, id: u64, stream: impl Read) {
    let mut reader = BufReader::new(stream);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let text = String::from_utf8_lossy(&buffer);
                if let Some(line) = text.split('\r').map(str::trim).filter(|l| !l.is_empty()).last() {
                    let _ = app.emit("job-line", JobLine { id, line: line.to_string() });
                }
            }
        }
    }
}

#[cfg(test)]
mod driver_tests {
    use super::*;

    fn request(action: &str, ids: &[&str]) -> JobRequest {
        JobRequest { action: action.into(), source: "driver".into(), ids: ids.iter().map(|s| s.to_string()).collect(), scope: None, file: None }
    }

    fn mentions(steps: &[Vec<String>], word: &str) -> bool {
        steps.iter().flatten().any(|part| part == word)
    }

    #[test]
    fn a_listed_branch_becomes_the_matching_privileged_action() {
        assert!(mentions(&plan(&request("install", &["590"])).unwrap(), "nvidia-install-cuda"));
        assert!(mentions(&plan(&request("install", &["nvidia-driver"])).unwrap(), "nvidia-install-repo"));
        assert!(mentions(&plan(&request("remove", &[])).unwrap(), "nvidia-remove"));
        assert!(mentions(&plan(&request("install", &["oneapi-mkl"])).unwrap(), "vendor-install"));
    }

    #[test]
    fn anything_not_in_the_closed_lists_is_refused() {
        assert!(plan(&request("install", &["999"])).is_err());
        assert!(plan(&request("install", &["590; reboot"])).is_err());
        assert!(plan(&request("install", &["bash"])).is_err());
        assert!(plan(&request("install", &[])).is_err());
        assert!(plan(&request("install", &["580", "590"])).is_err(), "one driver at a time");
    }
}
