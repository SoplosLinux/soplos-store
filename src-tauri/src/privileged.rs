// The only part of the app that runs as root. The window never runs as
// root: for APT operations it starts
//     pkexec /usr/bin/soplos-store --privileged <action> <args>
// and main() routes that here before Tauri starts. Only the fixed actions
// below exist, and every argument is validated again, so polkit's
// authorization cannot be used to run anything else.

use std::process::Command;

use crate::validate;

/// Entry point for `--privileged-refresh`, run as root through pkexec with
/// allow_active=yes. Exclusively runs `apt-get update` with no package arguments.
pub fn run_refresh(args: &[String]) -> i32 {
    let mut lang = None;
    let mut locale = None;
    let mut args = args;
    while let Some(first) = args.first() {
        if let Some(value) = first.strip_prefix("--lang=") {
            lang = Some(value).filter(|v| v.len() == 2 && v.chars().all(|c| c.is_ascii_lowercase()));
        } else if let Some(value) = first.strip_prefix("--locale=") {
            locale = Some(value).filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '@' | '-')));
        } else {
            break;
        }
        args = &args[1..];
    }
    let mut apt = Command::new("apt-get");
    apt.arg("-q=0").arg("update").env("DEBIAN_FRONTEND", "noninteractive");
    if let Some(locale) = locale {
        apt.env("LANG", locale);
    }
    if let Some(lang) = lang {
        apt.env("LANGUAGE", format!("{lang}:en"));
    }
    match apt.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("soplos-store: could not run apt-get update: {e}");
            1
        }
    }
}

pub fn run(args: &[String]) -> i32 {
    // Optional leading options with the language of the app, so apt writes
    // in it (see jobs.rs): --lang=xx and --locale=xx_XX.UTF-8.
    let mut lang = None;
    let mut locale = None;
    let mut args = args;
    while let Some(first) = args.first() {
        if let Some(value) = first.strip_prefix("--lang=") {
            lang = Some(value).filter(|v| v.len() == 2 && v.chars().all(|c| c.is_ascii_lowercase()));
        } else if let Some(value) = first.strip_prefix("--locale=") {
            locale = Some(value).filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '@' | '-')));
        } else {
            break;
        }
        args = &args[1..];
    }
    let Some((action, rest)) = args.split_first() else {
        eprintln!("soplos-store: missing privileged action");
        return 2;
    };
    let all_packages = !rest.is_empty() && rest.iter().all(|p| validate::apt_package(p));
    let apt_args: Vec<String> = match action.as_str() {
        // --no-remove: an install or upgrade never removes other packages
        // behind the user's back; apt stops instead.
        "apt-install" if all_packages => args_with(&["install", "-y", "--no-remove"], rest),
        "apt-remove" if all_packages => args_with(&["remove", "-y"], rest),
        "apt-upgrade" if all_packages => args_with(&["install", "-y", "--no-remove", "--only-upgrade"], rest),
        "apt-install-file" if rest.len() == 1 && validate::package_file(&rest[0], &["deb"]) => {
            args_with(&["install", "-y", "--no-remove"], rest)
        }
        // Only refreshes the package lists; installs and removes nothing.
        "apt-update" if rest.is_empty() => vec!["update".into()],
        "apt-autoremove" if rest.is_empty() => vec!["autoremove".into(), "-y".into()],
        "apt-clean" if rest.is_empty() => vec!["clean".into()],
        _ => {
            eprintln!("soplos-store: invalid privileged action or arguments");
            return 2;
        }
    };
    // -q=0: when its output is not a terminal apt turns quiet mode on by
    // itself and never prints the final "Done" of each step, so the log in
    // the window would end on "Reading package lists..." and look unfinished.
    let mut apt = Command::new("apt-get");
    apt.arg("-q=0").args(&apt_args).env("DEBIAN_FRONTEND", "noninteractive");
    if let Some(locale) = locale {
        apt.env("LANG", locale);
    }
    if let Some(lang) = lang {
        apt.env("LANGUAGE", format!("{lang}:en"));
    }
    match apt.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("soplos-store: could not run apt-get: {e}");
            1
        }
    }
}

fn args_with(fixed: &[&str], rest: &[String]) -> Vec<String> {
    fixed.iter().map(|s| s.to_string()).chain(rest.iter().cloned()).collect()
}
