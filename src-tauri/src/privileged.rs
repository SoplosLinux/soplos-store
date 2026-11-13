// The only part of the app that runs as root. The window never runs as
// root: for APT operations it starts
//     pkexec /usr/bin/soplos-store --privileged <action> <args>
// and main() routes that here before Tauri starts. Only the fixed actions
// below exist, and every argument is validated again, so polkit's
// authorization cannot be used to run anything else.

use std::process::Command;

use crate::{apt, nvidia, validate, vendor};

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
    // Driver actions run a script that is built into this binary. The only
    // thing the caller chooses is the branch or package, and it is accepted
    // only when it is in the closed list of nvidia.rs; any other value
    // falls through to "invalid privileged action" below.
    let script = match action.as_str() {
        "nvidia-install-repo" if rest.len() == 1 => nvidia::repo_script(&rest[0]).ok(),
        "nvidia-install-cuda" if rest.len() == 1 => nvidia::cuda_script(&rest[0]).ok(),
        "nvidia-remove" if rest.is_empty() => Some(nvidia::uninstall_script()),
        "vendor-install" if rest.len() == 1 => vendor::install_script(&rest[0]).ok(),
        _ => None,
    };
    if let Some(script) = script {
        return run_script(&script, lang, locale);
    }
    let all_packages = !rest.is_empty() && rest.iter().all(|p| validate::apt_package(p));
    let apt_args: Vec<String> = match action.as_str() {
        // --no-remove: an install or upgrade never removes other packages
        // behind the user's back; apt stops instead.
        "apt-install" if all_packages => args_with(&["install", "-y", "--no-remove"], rest),
        "apt-remove" if all_packages => args_with(&["remove", "-y"], rest),
        // Upgrades follow the plan of the solver instead of --no-remove: see upgrade_args.
        "apt-upgrade" if all_packages => match upgrade_args(rest) {
            Ok(Some(args)) => args,
            Ok(None) => return 0,
            Err(code) => return code,
        },
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
    // APT::Status-Fd=1: apt also writes "dlstatus:" and "pmstatus:" lines, the
    // same in every language, which the window reads for the progress of each
    // package (see updateProgress.ts).
    let mut apt = Command::new("apt-get");
    apt.arg("-q=0").args(["-o", "APT::Status-Fd=1"]).args(&apt_args).env("DEBIAN_FRONTEND", "noninteractive");
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

/// Runs one of the built-in driver scripts as root, in the language of the app.
fn run_script(script: &str, lang: Option<&str>, locale: Option<&str>) -> i32 {
    let mut shell = Command::new("bash");
    shell.arg("-c").arg(script).env("DEBIAN_FRONTEND", "noninteractive");
    if let Some(locale) = locale {
        shell.env("LANG", locale);
    }
    if let Some(lang) = lang {
        shell.env("LANGUAGE", format!("{lang}:en"));
    }
    match shell.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("soplos-store: could not run the driver script: {e}");
            1
        }
    }
}

/// Arguments for upgrading packages the way the plan of the solver says. The
/// helper makes the plan itself instead of trusting the window: a package that
/// is not in it is skipped, and the upgrade is refused when it would remove
/// anything the plan does not remove (a renamed library is replaced, nothing
/// else goes). Ok(None) means there is nothing to upgrade.
fn upgrade_args(requested: &[String]) -> Result<Option<Vec<String>>, i32> {
    let Some(plan) = apt::upgrade_plan() else {
        eprintln!("soplos-store: could not work out the upgrade");
        return Err(1);
    };
    let ids: Vec<String> = requested.iter().filter(|name| plan.updates.iter().any(|(n, _, _)| n == *name)).cloned().collect();
    if ids.is_empty() {
        eprintln!("soplos-store: nothing to upgrade");
        return Ok(None);
    }
    let Some(simulated) = apt::install_plan(&ids) else {
        eprintln!("soplos-store: could not work out the upgrade of {}", ids.join(" "));
        return Err(1);
    };
    if let Some((name, _)) = simulated.removed.iter().find(|(name, _)| !plan.removed.iter().any(|(n, _)| n == name)) {
        eprintln!("soplos-store: refused, the upgrade would also remove {name}");
        return Err(2);
    }
    Ok(Some(args_with(&["install", "-y", "-o", apt::SOLVER_OPTION, "--only-upgrade"], &ids)))
}

fn args_with(fixed: &[&str], rest: &[String]) -> Vec<String> {
    fixed.iter().map(|s| s.to_string()).chain(rest.iter().cloned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // None of these may run a script: they only check what the helper
    // refuses. A valid driver action would start a root script, so it is
    // never called here.
    fn refused(args: &[&str]) -> bool {
        run(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()) == 2
    }

    #[test]
    fn the_helper_refuses_branches_outside_the_closed_lists() {
        assert!(refused(&["nvidia-install-cuda", "999"]));
        assert!(refused(&["nvidia-install-cuda", "620; reboot"]));
        assert!(refused(&["nvidia-install-cuda"]));
        assert!(refused(&["nvidia-install-repo", "bash"]));
        assert!(refused(&["nvidia-install-repo", "nvidia-driver", "extra"]));
        assert!(refused(&["nvidia-remove", "anything"]));
        assert!(refused(&["vendor-install", "bash"]));
        assert!(refused(&["vendor-install", "oneapi-mkl; reboot"]));
        assert!(refused(&["vendor-install"]));
        assert!(refused(&["run-this-script", "/tmp/x.sh"]));
    }
}
