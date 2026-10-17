// Checks for every value that ends up as an argument of apt, flatpak,
// snap or pkexec. Commands are always built as argument lists (never a
// shell string), and these checks add a second barrier on top.

use std::path::Path;

/// Debian package name: lowercase letters, digits, + - . (policy 5.6.1).
pub fn apt_package(name: &str) -> bool {
    name.len() >= 2
        && name.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '+' | '-' | '.'))
}

/// Flatpak application ID such as org.gimp.GIMP.
pub fn flatpak_id(id: &str) -> bool {
    id.contains('.')
        && !id.starts_with('.')
        && !id.starts_with('-')
        && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Snap name: lowercase letters, digits and hyphens.
pub fn snap_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// fwupd device ID: 40 lowercase hex digits.
pub fn fwupd_device_id(id: &str) -> bool {
    id.len() == 40 && id.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// An existing absolute path to a file with one of the given extensions.
pub fn package_file(path: &str, extensions: &[&str]) -> bool {
    let file = Path::new(path);
    file.is_absolute()
        && file.is_file()
        && file
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| extensions.contains(&e.to_lowercase().as_str()))
}
