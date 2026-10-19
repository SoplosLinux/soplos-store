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

use crate::{apt, flatpak, fwupd, i18n, snap, validate};

const FLATHUB_REPO: &str = "https://dl.flathub.org/repo/flathub.flatpakrepo";

/// pkexec exit codes for a dismissed or refused password dialog.
const PKEXEC_CANCELLED: [i32; 2] = [126, 127];

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
            if command[0] == "#report" {
                total += report(&app, id, &command[1]);
                continue;
            }
            match run_step(&app, id, command) {
                Ok(0) => {}
                Ok(_) if tolerant => {}
                Ok(code) => {
                    success = false;
                    cancelled = command[0] == "pkexec" && PKEXEC_CANCELLED.contains(&code);
                    break;
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
        ("appimage", "install-file") => {
            crate::appimage::install_file(std::path::Path::new(&file))?;
            vec![vec!["#echo".to_string(), "OK".to_string()]]
        }

        // Firmware goes through fwupd, which asks for authorization itself.
        ("firmware", "update") => {
            need_ids(validate::fwupd_device_id)?;
            ids.iter().map(|id| strings(&["fwupdmgr", "update", id, "-y", "--no-reboot-check"])).collect()
        }
        ("firmware", "testing-on") => vec![strings(&["fwupdmgr", "enable-remote", "lvfs-testing", "-y"]), strings(&["fwupdmgr", "refresh", "--force"])],
        ("firmware", "testing-off") => vec![strings(&["fwupdmgr", "disable-remote", "lvfs-testing", "-y"])],

        // "Check now": every installed source, one after another, in the
        // same order as Soplos Welcome: APT first (it refreshes the package
        // lists and asks for the password), then Snap and Flatpak. After
        // each one a "#report" step lists what it found and says so when
        // there is nothing, so the log never ends on a bare heading. The
        // Updates page reads all three sources again when the job ends.
        ("all", "check") => {
            let heading = |key: &str| vec!["#echo".to_string(), i18n::t(key)];
            let mut steps: Vec<Vec<String>> = vec![heading("checklog.apt"), privileged_refresh()?, strings(&["#report", "apt"])];
            if snap::available() {
                steps.push(heading("checklog.snap"));
                steps.push(strings(&["#report", "snap"]));
            }
            if flatpak::available() {
                steps.push(heading("checklog.flatpak"));
                steps.push(strings(&["#report", "flatpak"]));
            }
            if fwupd::available() {
                steps.push(heading("checklog.firmware"));
                steps.push(vec!["?".into(), "fwupdmgr".into(), "refresh".into(), "--force".into()]);
                steps.push(strings(&["#report", "firmware"]));
            }
            steps
        }

        (source, action) => return Err(format!("unsupported job: {action} from {source}")),
    };
    Ok(steps)
}

/// Writes what one source has pending (one line per update) and a result
/// line, and returns how many updates it found.
fn report(app: &AppHandle, id: u64, source: &str) -> usize {
    let (label, lines): (&str, Vec<String>) = match source {
        "apt" => ("APT", apt::upgradable().into_iter().map(|(name, from, to)| format!("  {name}  {from} -> {to}")).collect()),
        "snap" => ("Snap", snap::updates().into_iter().map(|(name, version)| format!("  {name}  {version}")).collect()),
        "firmware" => ("Firmware", fwupd::updates().into_iter().map(|u| format!("  {}  {} -> {}", u.name, u.from, u.to)).collect()),
        _ => ("Flatpak", flatpak::updates().into_iter().map(|(app_id, _, version)| format!("  {app_id}  {version}")).collect()),
    };
    let count = lines.len();
    for line in lines {
        let _ = app.emit("job-line", JobLine { id, line });
    }
    let result = if count == 0 { i18n::t("checklog.none") } else { i18n::t_n("checklog.found", count) };
    let _ = app.emit("job-line", JobLine { id, line: format!("{label}: {result}") });
    count
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
