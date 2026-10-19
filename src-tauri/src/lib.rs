mod appimage;
mod apt;
mod cache;
mod catalog;
mod desktop;
mod checker;
mod flathub;
mod flatpak;
mod fwupd;
mod http;
mod i18n;
mod images;
mod jobs;
mod launcher;
mod models;
mod odrs;
mod privileged;
mod settings;
mod snap;
mod system;
mod text;
mod tray;
mod validate;

/// Entry point for `--privileged <action> <args>`, run as root through
/// pkexec. Returns the exit code.
pub fn run_privileged(args: &[String]) -> i32 {
    privileged::run(args)
}

/// Entry point for `--privileged-refresh`, run as root through pkexec with
/// allow_active=yes. Exclusively runs `apt-get update`.
pub fn run_privileged_refresh(args: &[String]) -> i32 {
    privileged::run_refresh(args)
}

/// Entry point for `--tray`, the update notifier started with the session.
pub fn run_tray() -> i32 {
    tray::run()
}

/// Entry point for `--check-updates`, run by the systemd user timer.
pub fn run_check() -> i32 {
    checker::run()
}

use serde::Serialize;

#[derive(Serialize)]
pub struct EnvironmentInfo {
    desktop: String,
    session: String,
}

/// Desktop environment and display protocol for the sidebar footer,
/// e.g. "XFCE" and "X11".
#[tauri::command]
fn get_environment_info() -> EnvironmentInfo {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or("")
        .to_string();
    let session = match std::env::var("XDG_SESSION_TYPE")
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "x11" => "X11".to_string(),
        "wayland" => "Wayland".to_string(),
        _ => String::new(),
    };
    EnvironmentInfo { desktop, session }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    catalog::prewarm();
    launcher::remember_startup_file(launcher::package_file_from_args(std::env::args()));
    launcher::remember_startup_updates(launcher::wants_updates(std::env::args()));
    tauri::Builder::default()
        // Registered first so a second launch only focuses the open window,
        // passing along the package file it was opened with, if any.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            use tauri::{Emitter, Manager};
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            if launcher::wants_updates(args.iter().cloned()) {
                let _ = app.emit("open-updates", ());
            }
            if let Some(path) = launcher::package_file_from_args(args) {
                let _ = app.emit("open-package", path);
            }
        }))
        // The tray must be running while the app is open (it exits by
        // itself if one already is, or if automatic checks are off).
        .setup(|_| {
            let _ = tray::start_tray();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_environment_info,
            catalog::get_explore,
            catalog::get_category,
            catalog::search_apps,
            catalog::get_app_detail,
            catalog::get_installed,
            catalog::get_updates,
            catalog::get_sources_status,
            images::fetch_image,
            launcher::launch_app,
            launcher::open_url,
            launcher::take_startup_file,
            launcher::inspect_package,
            jobs::start_job,
            i18n::set_language,
            catalog::get_apt_status,
            catalog::reload_apt,
            launcher::take_startup_updates,
            settings::save_settings,
            tray::start_tray,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Soplos Store");
}
