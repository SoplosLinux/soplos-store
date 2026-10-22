// The Printers section of the Drivers page: the printers plugged in by USB
// and what the Store offers to install for each brand. The drivers come from
// Debian, where printer makers and the community keep them; the model
// specific ones that only the maker publishes are opened from the page as a
// link to its official downloads.

use std::path::Path;

use serde::Serialize;

use crate::gpu::{packages_status, PackageSet};

/// Every set of the page and the Debian packages it installs.
pub const SETS: [(&str, &[&str]); 10] = [
    ("print-cups", &["cups", "cups-filters", "ipp-usb", "system-config-printer"]),
    ("print-pdf", &["printer-driver-cups-pdf"]),
    ("print-gutenprint", &["printer-driver-gutenprint"]),
    ("print-hp", &["hplip", "hplip-gui", "printer-driver-postscript-hp"]),
    ("print-brother", &["printer-driver-brlaser"]),
    ("print-epson", &["printer-driver-escpr"]),
    ("print-xerox", &["printer-driver-fujixerox", "printer-driver-foo2zjs"]),
    ("print-ricoh", &["printer-driver-sag-gdi"]),
    ("print-oki", &["printer-driver-oki"]),
    ("print-samsung", &["printer-driver-splix"]),
];

/// The sets and whether each one is complete.
#[tauri::command]
pub async fn get_printer_packages() -> Vec<PackageSet> {
    tauri::async_runtime::spawn_blocking(|| {
        SETS.iter()
            .map(|(id, packages)| PackageSet {
                id: (*id).into(),
                packages: packages.iter().map(|p| p.to_string()).collect(),
                installed: packages_status(packages).0 == "installed",
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// A set a page asks about: its id and the Debian packages it is made of.
#[derive(serde::Deserialize)]
pub struct SetRequest {
    pub id: String,
    pub packages: Vec<String>,
}

/// Whether each set a page names is complete. The pages of the Drivers section
/// carry their own lists of packages, so this answers for any of them; a name
/// that is not a valid package name is left out of the answer.
#[tauri::command]
pub async fn get_package_sets(sets: Vec<SetRequest>) -> Vec<PackageSet> {
    tauri::async_runtime::spawn_blocking(move || {
        sets.into_iter()
            .filter(|set| !set.packages.is_empty() && set.packages.iter().all(|p| crate::validate::apt_package(p)))
            .map(|set| {
                let names: Vec<&str> = set.packages.iter().map(String::as_str).collect();
                let installed = packages_status(&names).0 == "installed";
                PackageSet { id: set.id, packages: set.packages, installed }
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// A printer found on the computer.
#[derive(Clone, Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrinterInfo {
    /// Id of the brand on the page, empty when it is not one of them.
    pub brand: String,
    pub vendor: String,
    pub model: String,
}

/// The brand a maker name belongs to. Words are compared whole, so "hp" does
/// not match inside another word.
pub fn brand_of(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).collect();
    let has = |names: &[&str]| words.iter().any(|w| names.contains(w));
    if has(&["hp", "hewlett", "packard"]) {
        "hp"
    } else if has(&["brother"]) {
        "brother"
    } else if has(&["epson", "seiko"]) {
        "epson"
    } else if has(&["canon"]) {
        "canon"
    } else if has(&["xerox"]) {
        "xerox"
    } else if has(&["ricoh", "lanier", "savin"]) {
        "ricoh"
    } else if has(&["kyocera"]) {
        "kyocera"
    } else if has(&["lexmark"]) {
        "lexmark"
    } else if has(&["oki", "okidata"]) {
        "oki"
    } else if has(&["konica", "minolta"]) {
        "konicaminolta"
    } else if has(&["samsung"]) {
        "samsung"
    } else if has(&["pantum"]) {
        "pantum"
    } else {
        ""
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).map(|s| s.trim().to_string()).unwrap_or_default()
}

/// The USB printers: the devices with an interface of the printer class (07),
/// read from sysfs, so it needs no tool and no CUPS.
fn usb_printers() -> Vec<PrinterInfo> {
    let root = Path::new("/sys/bus/usb/devices");
    let mut found: Vec<PrinterInfo> = Vec::new();
    for entry in std::fs::read_dir(root).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some((device, _)) = name.split_once(':') else {
            continue;
        };
        if read(&entry.path().join("bInterfaceClass")) != "07" {
            continue;
        }
        let device_dir = root.join(device);
        let vendor = read(&device_dir.join("manufacturer"));
        let model = read(&device_dir.join("product"));
        if model.is_empty() && vendor.is_empty() {
            continue;
        }
        let info = PrinterInfo { brand: brand_of(&format!("{vendor} {model}")).into(), vendor, model };
        if !found.contains(&info) {
            found.push(info);
        }
    }
    found
}

/// The printers plugged in by USB.
#[tauri::command]
pub async fn get_printers() -> Vec<PrinterInfo> {
    tauri::async_runtime::spawn_blocking(usb_printers).await.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_brand_comes_from_the_name_the_printer_gives() {
        assert_eq!(brand_of("HP LaserJet Pro M404dn"), "hp");
        assert_eq!(brand_of("Hewlett-Packard DeskJet 2700"), "hp");
        assert_eq!(brand_of("Brother HL-L2350DW"), "brother");
        assert_eq!(brand_of("EPSON L3250 Series"), "epson");
        assert_eq!(brand_of("Seiko Epson Corp."), "epson");
        assert_eq!(brand_of("Canon PIXMA TS3350"), "canon");
        assert_eq!(brand_of("KONICA MINOLTA bizhub"), "konicaminolta");
        assert_eq!(brand_of("OKI B412"), "oki");
        assert_eq!(brand_of("Samsung M2020"), "samsung");
    }

    #[test]
    fn a_word_that_only_contains_a_brand_is_not_that_brand() {
        assert_eq!(brand_of("Chip Printer"), "");
        assert_eq!(brand_of("Joki Labels"), "");
        assert_eq!(brand_of(""), "");
    }

    #[test]
    fn a_page_cannot_ask_about_a_name_that_is_not_a_package() {
        let ok = SetRequest { id: "a".into(), packages: vec!["sane-utils".into()] };
        let bad = SetRequest { id: "b".into(), packages: vec!["sane-utils; reboot".into()] };
        let valid = |set: &SetRequest| !set.packages.is_empty() && set.packages.iter().all(|p| crate::validate::apt_package(p));
        assert!(valid(&ok));
        assert!(!valid(&bad));
    }

    #[test]
    fn every_set_is_made_of_names_the_apt_job_accepts() {
        for (id, packages) in SETS {
            assert!(!packages.is_empty(), "{id} has packages");
            for package in packages {
                assert!(crate::validate::apt_package(package), "{package} is an apt name");
            }
        }
    }
}
