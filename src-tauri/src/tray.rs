// Update notifier: `soplos-store --tray`, started with the session from
// /etc/xdg/autostart (and by the app when it opens). It has no window. The
// check itself is done by the systemd user timer (see checker.rs); this
// only reads the stored result and shows a tray icon and a desktop
// notification while there are updates. Clicking either opens the app on
// Updates. If the timer has not run for longer than the frequency chosen
// in Settings, the tray checks by itself.

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::StandardItem;

use crate::{cache, checker, i18n, settings};

/// Waits a little after login so the session start is not slowed down.
const FIRST_READ_DELAY: Duration = Duration::from_secs(15);

/// How often the stored result is read again (a cheap file read).
const READ_EVERY: Duration = Duration::from_secs(300);

enum Message {
    CheckNow,
    Quit,
}

/// Menu and notification texts, read from the dictionaries (see i18n.rs).
#[derive(Clone)]
struct Texts {
    open: String,
    check: String,
    quit: String,
}

impl Texts {
    fn load() -> Texts {
        Texts { open: i18n::t("tray.open"), check: i18n::t("tray.check"), quit: i18n::t("tray.quit") }
    }
}

fn available(count: usize) -> String {
    i18n::t_n("tray.available", count)
}

struct UpdatesTray {
    count: usize,
    texts: Texts,
    sender: Sender<Message>,
}

impl ksni::Tray for UpdatesTray {
    fn id(&self) -> String {
        "soplos-store".into()
    }

    fn title(&self) -> String {
        "Soplos Store".into()
    }

    // The icon theme's own update icon, drawn to fit the panel.
    fn icon_name(&self) -> String {
        "software-update-available".into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip { title: "Soplos Store".into(), description: available(self.count), ..Default::default() }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        open_updates();
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            StandardItem { label: self.texts.open.clone(), activate: Box::new(|_: &mut Self| open_updates()), ..Default::default() }.into(),
            StandardItem {
                label: self.texts.check.clone(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.sender.send(Message::CheckNow);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: self.texts.quit.clone(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.sender.send(Message::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Opens the app on the Updates section (or brings the open window there).
fn open_updates() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).arg("--updates").spawn();
    }
}

/// The stored count; checks first when there is none or it is too old, or
/// when asked to.
fn current_count(interval: Duration, force: bool) -> usize {
    match checker::read() {
        Some(stored) if !force && checker::age().is_some_and(|age| age < interval) => stored.count,
        _ => checker::check_and_store(),
    }
}

fn notify(texts: Texts, count: usize) {
    std::thread::spawn(move || {
        let shown = notify_rust::Notification::new()
            .appname("Soplos Store")
            .summary("Soplos Store")
            .body(&available(count))
            .icon("soplos-store")
            .action("default", &texts.open)
            .show();
        if let Ok(handle) = shown {
            handle.wait_for_action(|action| {
                if action == "default" {
                    open_updates();
                }
            });
        }
    });
}

/// One notifier per user: a second `--tray` exits at once.
fn take_lock() -> bool {
    let lock: PathBuf = cache::dir("").join("tray.pid");
    if let Ok(pid) = std::fs::read_to_string(&lock) {
        let cmdline = std::fs::read(format!("/proc/{}/cmdline", pid.trim())).unwrap_or_default();
        if String::from_utf8_lossy(&cmdline).contains("--tray") {
            return false;
        }
    }
    std::fs::write(&lock, std::process::id().to_string()).is_ok()
}

pub fn run() -> i32 {
    if !settings::load().auto_check {
        eprintln!("soplos-store tray: automatic checks are off in Settings, exiting");
        return 0;
    }
    if !take_lock() {
        eprintln!("soplos-store tray: already running, exiting");
        return 0;
    }
    eprintln!("soplos-store tray: started, first read in {} seconds", FIRST_READ_DELAY.as_secs());
    let texts = Texts::load();
    let (sender, receiver) = mpsc::channel();
    let mut tray: Option<Handle<UpdatesTray>> = None;
    let mut notified = 0;
    let mut wait = FIRST_READ_DELAY;

    loop {
        let mut force = false;
        match receiver.recv_timeout(wait) {
            Ok(Message::Quit) => break,
            Ok(Message::CheckNow) => force = true,
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {}
        }
        let config = settings::load();
        if !config.auto_check {
            break;
        }
        let count = current_count(config.interval(), force);
        eprintln!("soplos-store tray: {count} updates");
        if count == 0 {
            // No icon at all while everything is up to date.
            if let Some(handle) = tray.take() {
                handle.shutdown().wait();
            }
            notified = 0;
        } else {
            match &tray {
                Some(handle) => {
                    handle.update(|t| t.count = count);
                }
                None => match (UpdatesTray { count, texts: texts.clone(), sender: sender.clone() }).spawn() {
                    Ok(handle) => tray = Some(handle),
                    // No StatusNotifierItem host (e.g. GNOME without the
                    // AppIndicator extension): the notification still shows.
                    Err(error) => eprintln!("soplos-store tray: could not show the tray icon: {error}"),
                },
            }
            // A notification only when there are more updates than last time.
            if config.notify && count > notified {
                notify(texts.clone(), count);
            }
            notified = count;
        }
        wait = READ_EVERY;
    }

    if let Some(handle) = tray {
        handle.shutdown().wait();
    }
    let _ = std::fs::remove_file(cache::dir("").join("tray.pid"));
    0
}

/// Starts the notifier from the window (e.g. after enabling automatic
/// checks again); it exits by itself if one is already running.
#[tauri::command]
pub fn start_tray() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new(exe)
        .arg("--tray")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
