// GPU detection, ported from Soplos Welcome 3.0 (commands/drivers.rs). It reads
// lspci, tells the vendor and model of each graphics card and what driver
// fits it, and whether that driver is already installed.

use std::process::Command;

use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub(crate) vendor: String,
    pub(crate) model: String,
    gpu_type: String,
    recommended_driver: Option<String>,
    driver_status: String,
    installed_driver_version: Option<String>,
}

pub(crate) fn is_package_installed(package: &str) -> bool {
    Command::new("dpkg-query")
        .args(["-W", "-f=${Status}", package])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains("install ok installed"))
        .unwrap_or(false)
}

pub(crate) fn get_lspci_output() -> String {
    Command::new("lspci")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

fn detect_installed_nvidia_version() -> Option<String> {
    if let Ok(output) = Command::new("nvidia-smi")
        .args(["--query-gpu=driver_version", "--format=csv,noheader"])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(major) = text.trim().split('.').next() {
                if !major.is_empty() && major.chars().all(|c| c.is_ascii_digit()) {
                    return Some(major.to_string());
                }
            }
        }
    }

    if let Ok(output) = Command::new("dpkg").arg("-l").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if !line.starts_with("ii") {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            if let Some(rest) = parts[1].strip_prefix("nvidia-driver-") {
                if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                    return Some(rest.to_string());
                }
            }
        }
    }

    None
}

// Marker substrings for GPUs of the Turing generation or newer — the only
// cards the 620 driver branch supports. Installing them on an older
// card leaves the machine with no working module, and since the NVIDIA
// package blacklists nouveau, with no graphics driver at all. Ported as-is
// from hardware_detector.py's NVIDIA_TURING_PLUS_MARKERS.
const NVIDIA_TURING_PLUS_MARKERS: &[&str] = &[
    "rtx",
    "gtx 16",
    "gtx16",
    "mx450",
    "mx 450",
    "mx550",
    "mx 550",
    "mx570",
    "mx 570",
    "t400",
    "t500",
    "t550",
    "t600",
    "t1000",
    "t1200",
    "t2000",
    "tesla t4",
];

// Mirrors Python's `\b` regex word boundary: a plain substring check would
// let a short marker like "ati" match inside an unrelated word like
// "Corporation", or "t550" match inside "gtx550" (a Fermi card, not Turing).
// Ported from hardware_detector.py, which hit exactly these two collisions.
pub(crate) fn contains_word_boundary(haystack: &str, needle: &str) -> bool {
    let is_word_char = |c: char| c.is_alphanumeric();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        let abs_pos = start + pos;
        let before_ok = haystack[..abs_pos].chars().last().map_or(true, |c| !is_word_char(c));
        let after_idx = abs_pos + needle.len();
        let after_ok = haystack[after_idx..].chars().next().map_or(true, |c| !is_word_char(c));
        if before_ok && after_ok {
            return true;
        }
        start = abs_pos + 1;
        if start >= haystack.len() {
            break;
        }
    }
    false
}

pub(crate) fn is_nvidia_turing_or_newer(model: &str) -> bool {
    let model_lower = model.to_lowercase();
    // "gtx 16" is the start of a model number (GTX 1650, 1660 Ti): as a whole
    // word it never matched, and these Turing cards were sent to nouveau.
    ["gtx 16", "gtx16"].iter().any(|m| model_lower.contains(m))
        || NVIDIA_TURING_PLUS_MARKERS.iter().any(|marker| contains_word_boundary(&model_lower, marker))
}

fn extract_bracketed_model(line: &str) -> Option<String> {
    let start = line.rfind('[')?;
    let end = line[start..].find(']')? + start;
    Some(line[start + 1..end].trim().to_string())
}

// Fallback regex-free extraction for lspci lines with no bracketed marketing
// name (rare, but hardware_detector.py hits it and has a per-vendor fallback
// for each). Looks for the first occurrence of any of `patterns` and returns
// everything from that point to the next '(' or end of line.
fn extract_model_fallback(line: &str, patterns: &[&str]) -> Option<String> {
    let lower = line.to_lowercase();
    for pattern in patterns {
        if let Some(pos) = lower.find(pattern) {
            let rest = &line[pos..];
            let end = rest.find('(').unwrap_or(rest.len());
            let candidate = rest[..end].trim();
            // The UI prefixes the vendor itself ("{vendor} {model}"), so a
            // model that already starts with it rendered as "Intel Intel ...".
            let candidate = ["intel ", "amd ", "nvidia "]
                .iter()
                .find(|v| candidate.to_lowercase().starts_with(*v))
                .map(|v| candidate[v.len()..].trim())
                .unwrap_or(candidate);
            if !candidate.is_empty() {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

// Ported from hardware_detector.py's _recommend_nvidia_driver: the same
// branch cutoffs, in the same order, for the same reasons documented there.
fn recommend_nvidia_driver(model: &str) -> String {
    let model_lower = model.to_lowercase();

    if is_nvidia_turing_or_newer(&model_lower) {
        return "nvidia-driver-620".to_string();
    }
    let pascal_maxwell = [
        "gtx 10", "gtx10", "gtx 1080", "gtx 1070", "gtx 1060", "gtx 1050", "gt 1030", "mx350",
        "mx 350", "mx330", "mx 330", "mx250", "mx 250", "mx150", "mx 150", "gtx 9", "gtx 980",
        "gtx 970", "gtx 960", "gtx 950", "gtx 8", "gtx 880", "gtx 870", "gtx 860", "gtx 850",
        "920m", "930m", "940m", "920mx", "930mx", "940mx", "910m", "820m", "840m", "960m",
        "970m", "980m",
    ];
    if pascal_maxwell.iter().any(|s| model_lower.contains(s)) {
        return "nvidia-driver-580".to_string();
    }
    let kepler = [
        "gtx 7", "gtx 780", "gtx 770", "gtx 760", "gtx 750", "gtx 6", "gtx 680", "gtx 670",
        "gtx 660", "gtx 650", "gt 710", "gt 720", "gt 730", "gt 740", "mx230", "mx 230",
        "mx130", "mx 130", "mx110", "mx 110", "650m", "750m",
    ];
    if kepler.iter().any(|s| model_lower.contains(s)) {
        return "nvidia-tesla-470-driver".to_string();
    }
    let fermi = [
        "gtx 580", "gtx 570", "gtx 560", "gtx 550", "gtx 480", "gtx 470", "gtx 460", "gtx 450",
        "gtx 555", "gtx 520", "gtx 510", "gt 640", "gt 630", "gt 620", "gt 610", "gt 545",
        "gt 530", "gt 520", "gt 440", "gt 430", "gts 450",
    ];
    if fermi.iter().any(|s| model_lower.contains(s)) {
        return "nvidia-legacy-390xx-driver".to_string();
    }
    let tesla_gen = [
        "geforce 8", "geforce 9", "geforce 2", "geforce 3", "8400", "8600", "8800", "9400",
        "9500", "9600", "9800", "gts ", "320m", "330m", "quadro fx", "quadro nvs", "nforce",
    ];
    if tesla_gen.iter().any(|s| model_lower.contains(s)) {
        return "nouveau".to_string();
    }
    let professional = ["quadro", "tesla", "a100", "a40", "a30", "a10", "rtx a"];
    if professional.iter().any(|s| model_lower.contains(s)) {
        return "nvidia-driver".to_string();
    }
    // Unrecognised card: never fall back to a proprietary branch here. The
    // Debian nvidia-driver package only covers Maxwell to Ada, so anything
    // older would get a module that cannot build while the package
    // blacklists nouveau, leaving the machine with no driver at all.
    "nouveau".to_string()
}

/// Returns ("installed" | "partial" | "missing", missing_packages). Ported
/// from hardware_detector.py's _packages_status.
pub(crate) fn packages_status(required: &[&str]) -> (&'static str, Vec<String>) {
    let missing: Vec<String> = required.iter().filter(|p| !is_package_installed(p)).map(|s| s.to_string()).collect();
    if missing.is_empty() {
        ("installed", missing)
    } else if missing.len() == required.len() {
        ("missing", missing)
    } else {
        ("partial", missing)
    }
}

pub fn detect_gpus() -> Vec<GpuInfo> {
    detect_gpus_from_lspci(&get_lspci_output())
}

/// The graphics cards of this machine, with the driver that suits each one
/// and whether it is installed. It runs lspci and dpkg, so not on the UI thread.
#[tauri::command]
pub async fn get_gpus() -> Vec<GpuInfo> {
    tauri::async_runtime::spawn_blocking(detect_gpus).await.unwrap_or_default()
}

// Split out so scan_hardware() (hardware_scan.rs) can reuse the lspci output
// it already fetched once, instead of detect_gpus() spawning its own
// redundant second `lspci` process on every hardware scan.
pub(crate) fn detect_gpus_from_lspci(lspci: &str) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let mut nvidia_installed_version: Option<String> = None;
    let mut nvidia_checked = false;

    for line in lspci.lines() {
        let lower = line.to_lowercase();
        if !["vga", "3d", "display"].iter().any(|k| lower.contains(k)) {
            continue;
        }

        // "virtio" alone, not "virtio gpu": QEMU/KVM's virtio-gpu reports as
        // "Red Hat, Inc. Virtio 1.0 GPU" in lspci — the version number in the
        // middle means "virtio gpu" as a contiguous phrase never matches.
        // Verified against a real Proxmox/QEMU VM.
        if ["vmware svga", "virtualbox graphics", "vbox graphics", "qxl paravirtual", "bochs vbe", "virtio", "red hat"]
            .iter()
            .any(|k| lower.contains(k))
        {
            let model = if lower.contains("vmware") {
                "VMware SVGA II"
            } else if lower.contains("virtualbox") || lower.contains("vbox") {
                "VirtualBox VGA"
            } else if lower.contains("virtio") {
                "VirtIO GPU"
            } else if lower.contains("qxl") || lower.contains("red hat") {
                "QXL (QEMU/KVM)"
            } else {
                "Virtual GPU"
            };
            gpus.push(GpuInfo {
                vendor: "Virtual".to_string(),
                model: model.to_string(),
                gpu_type: "Virtual".to_string(),
                recommended_driver: None,
                driver_status: "installed".to_string(),
                installed_driver_version: None,
            });
        } else if contains_word_boundary(&lower, "nvidia") {
            let model = extract_bracketed_model(line)
                .or_else(|| extract_model_fallback(line, &["geforce rtx", "geforce gtx", "geforce", "nvidia"]))
                .unwrap_or_else(|| "NVIDIA Unknown".to_string());
            let recommended = recommend_nvidia_driver(&model);
            if !nvidia_checked {
                nvidia_installed_version = detect_installed_nvidia_version();
                nvidia_checked = true;
            }
            let status = if recommended == "nouveau" {
                if is_package_installed("xserver-xorg-video-nouveau") { "installed" } else { "missing" }
            } else if let Some(ver) = recommended.strip_prefix("nvidia-driver-") {
                match &nvidia_installed_version {
                    Some(installed) if installed == ver => "installed",
                    Some(_) => "different_version",
                    None => "missing",
                }
            } else if is_package_installed(&recommended) {
                "installed"
            } else if nvidia_installed_version.is_some() {
                "different_version"
            } else {
                "missing"
            };
            gpus.push(GpuInfo {
                vendor: "NVIDIA".to_string(),
                model,
                gpu_type: "Dedicated".to_string(),
                recommended_driver: Some(recommended),
                driver_status: status.to_string(),
                installed_driver_version: nvidia_installed_version.clone(),
            });
        } else if contains_word_boundary(&lower, "amd") || contains_word_boundary(&lower, "ati") || contains_word_boundary(&lower, "radeon") {
            let model = extract_bracketed_model(line)
                .or_else(|| extract_model_fallback(line, &["radeon rx", "radeon r", "radeon", "amd"]))
                .unwrap_or_else(|| "AMD Unknown".to_string());
            let required = AMD_PACKAGES;
            let (status, _missing) = packages_status(&required);
            gpus.push(GpuInfo {
                vendor: "AMD".to_string(),
                model,
                gpu_type: "Dedicated/Integrated".to_string(),
                recommended_driver: Some(required.join(" ")),
                driver_status: status.to_string(),
                installed_driver_version: None,
            });
        } else if contains_word_boundary(&lower, "intel") {
            let model = extract_bracketed_model(line)
                .or_else(|| extract_model_fallback(line, &["uhd graphics", "hd graphics", "iris xe", "iris", "intel"]))
                .unwrap_or_else(|| "Intel Unknown".to_string());
            let required = INTEL_PACKAGES;
            let (status, _missing) = packages_status(&required);
            gpus.push(GpuInfo {
                vendor: "Intel".to_string(),
                model,
                gpu_type: "Integrated".to_string(),
                recommended_driver: Some(required.join(" ")),
                driver_status: status.to_string(),
                installed_driver_version: None,
            });
        }
    }

    gpus
}


/// What an AMD card needs from Debian: its firmware and the open-source
/// stack (Mesa, Vulkan and the Xorg drivers).
pub const AMD_PACKAGES: [&str; 5] = ["firmware-amd-graphics", "libgl1-mesa-dri", "libglx-mesa0", "mesa-vulkan-drivers", "xserver-xorg-video-all"];

/// What an Intel card needs from Debian: the media driver for hardware video
/// and the Vulkan stack of Mesa.
pub const INTEL_PACKAGES: [&str; 2] = ["intel-media-va-driver", "mesa-vulkan-drivers"];

/// The guest tools of VMware: the daemon and the desktop integration
/// (shared clipboard, display resizing).
pub const VMWARE_PACKAGES: [&str; 2] = ["open-vm-tools", "open-vm-tools-desktop"];

/// The guest agent of QEMU and the SPICE agent that virt-manager uses for the
/// clipboard and the display of a KVM guest.
pub const QEMU_PACKAGES: [&str; 2] = ["qemu-guest-agent", "spice-vdagent"];

/// virt-manager with what it needs to run virtual machines on this computer:
/// the QEMU emulator and the libvirt daemon.
pub const VIRTMANAGER_PACKAGES: [&str; 3] = ["virt-manager", "qemu-system-x86", "libvirt-daemon-system"];

/// GNOME Boxes, the simple way to run virtual machines.
pub const BOXES_PACKAGES: [&str; 1] = ["gnome-boxes"];

/// ROCm of Debian: OpenCL and HIP runtime and the tools to read the card.
pub const AMD_ROCM_PACKAGES: [&str; 4] = ["rocm-opencl-icd", "rocminfo", "rocm-smi", "libamdhip64-6"];

/// What it takes to build HIP programs.
pub const AMD_HIP_PACKAGES: [&str; 5] = ["hipcc", "libamdhip64-dev", "libhsa-runtime-dev", "rocm-cmake", "hipify-clang"];

/// The ROCm math libraries.
pub const AMD_MATH_PACKAGES: [&str; 5] = ["librocblas-dev", "librocfft-dev", "librocsparse-dev", "librocsolver-dev", "libhipblas-dev"];

/// Tools to see what the card is doing.
pub const AMD_TOOLS_PACKAGES: [&str; 4] = ["radeontop", "nvtop", "vulkan-tools", "clinfo"];

/// Compute on Intel graphics: OpenCL and Level Zero.
pub const INTEL_COMPUTE_PACKAGES: [&str; 4] = ["intel-opencl-icd", "libze-intel-gpu1", "libze1", "clinfo"];

/// Hardware video on Intel graphics: oneVPL and the VA-API check tool.
pub const INTEL_MEDIA_PACKAGES: [&str; 3] = ["libvpl2", "libvpl-dev", "vainfo"];

/// Tools to see what the Intel card is doing.
pub const INTEL_TOOLS_PACKAGES: [&str; 3] = ["intel-gpu-tools", "nvtop", "vulkan-tools"];

/// A set of Debian packages that make up the driver or the guest tools of
/// one brand.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageSet {
    pub id: String,
    pub packages: Vec<String>,
    pub installed: bool,
}

/// The package sets of the brands other than NVIDIA (which has branches, see
/// nvidia.rs) and whether each one is complete, for the cards of the Drivers
/// page. Anyone can look at them, with or without that hardware. VirtualBox
/// has none: Debian testing carries no guest package for it.
#[tauri::command]
pub async fn get_driver_packages() -> Vec<PackageSet> {
    tauri::async_runtime::spawn_blocking(|| {
        let set = |id: &str, packages: &[&str]| PackageSet {
            id: id.into(),
            packages: packages.iter().map(|p| p.to_string()).collect(),
            installed: packages_status(packages).0 == "installed",
        };
        vec![
            set("amd", &AMD_PACKAGES),
            set("intel", &INTEL_PACKAGES),
            set("vmware", &VMWARE_PACKAGES),
            set("qemu", &QEMU_PACKAGES),
            // Proxmox runs its guests on QEMU: they use the same agent.
            set("proxmox", &QEMU_PACKAGES),
            set("virtmanager", &VIRTMANAGER_PACKAGES),
            set("boxes", &BOXES_PACKAGES),
            set("amd-rocm", &AMD_ROCM_PACKAGES),
            set("amd-hip", &AMD_HIP_PACKAGES),
            set("amd-math", &AMD_MATH_PACKAGES),
            set("amd-tools", &AMD_TOOLS_PACKAGES),
            set("intel-compute", &INTEL_COMPUTE_PACKAGES),
            set("intel-media", &INTEL_MEDIA_PACKAGES),
            set("intel-tools", &INTEL_TOOLS_PACKAGES),
        ]
        .into_iter()
        // The stacks of the makers, from their own repositories (vendor.rs).
        .chain(crate::vendor::ITEMS.iter().map(|(id, packages)| set(id, packages)))
        .collect()
    })
    .await
    .unwrap_or_default()
}

/// Whether each NVIDIA driver branch the Store offers is the one installed.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NvidiaBranch {
    pub id: String,
    pub installed: bool,
}

/// The state of every branch in nvidia.rs: the CUDA-repository ones are told
/// apart by the version of the driver in use, the Debian ones by their package.
#[tauri::command]
pub async fn get_nvidia_branches() -> Vec<NvidiaBranch> {
    tauri::async_runtime::spawn_blocking(|| {
        let version = detect_installed_nvidia_version();
        let cuda = crate::nvidia::CUDA_BRANCHES.iter().map(|id| NvidiaBranch { id: id.to_string(), installed: version.as_deref() == Some(*id) });
        let repo = crate::nvidia::REPO_PACKAGES.iter().map(|id| NvidiaBranch { id: id.to_string(), installed: is_package_installed(id) });
        cuda.chain(repo).collect()
    })
    .await
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RTX_3060: &str = "01:00.0 VGA compatible controller: NVIDIA Corporation GA106 [GeForce RTX 3060] (rev a1)";
    const GTX_960: &str = "01:00.0 VGA compatible controller: NVIDIA Corporation GM206 [GeForce GTX 960] (rev a1)";
    const GTX_650: &str = "01:00.0 VGA compatible controller: NVIDIA Corporation GK107 [GeForce GTX 650] (rev a1)";
    const FERMI: &str = "01:00.0 VGA compatible controller: NVIDIA Corporation GF110 [GeForce GTX 580] (rev a1)";
    const RX_6600: &str = "03:00.0 VGA compatible controller: Advanced Micro Devices, Inc. [AMD/ATI] Navi 23 [Radeon RX 6600] (rev c7)";
    const INTEL: &str = "00:02.0 VGA compatible controller: Intel Corporation Alder Lake-P GT2 [Iris Xe Graphics] (rev 0c)";
    const VMWARE: &str = "00:0f.0 VGA compatible controller: VMware SVGA II Adapter";

    fn one(line: &str) -> GpuInfo {
        let found = detect_gpus_from_lspci(line);
        assert_eq!(found.len(), 1, "{line}");
        found.into_iter().next().unwrap()
    }

    #[test]
    fn every_vendor_is_told_apart() {
        assert_eq!(one(RTX_3060).vendor, "NVIDIA");
        assert_eq!(one(RX_6600).vendor, "AMD");
        assert_eq!(one(INTEL).vendor, "Intel");
        assert_eq!(one(VMWARE).vendor, "Virtual");
        assert_eq!(one(RTX_3060).model, "GeForce RTX 3060");
        assert_eq!(one(INTEL).model, "Iris Xe Graphics");
    }

    #[test]
    fn the_nvidia_branch_follows_the_generation_of_the_card() {
        // Turing and newer take the newest branch, Maxwell and Pascal the 580,
        // and the older ones the legacy branches or nouveau.
        assert_eq!(recommend_nvidia_driver("GeForce RTX 3060"), "nvidia-driver-620");
        assert_eq!(recommend_nvidia_driver("GeForce GTX 1650"), "nvidia-driver-620");
        assert_eq!(recommend_nvidia_driver("TU116 [GeForce GTX 1660 Ti]"), "nvidia-driver-620");
        assert_eq!(recommend_nvidia_driver("GeForce GTX 960"), "nvidia-driver-580");
        assert_eq!(recommend_nvidia_driver("GeForce GTX 650"), "nvidia-tesla-470-driver");
        assert_eq!(recommend_nvidia_driver("GeForce GTX 580"), "nvidia-legacy-390xx-driver");
        // An unknown card never gets a proprietary branch.
        assert_eq!(recommend_nvidia_driver("Something Unheard Of"), "nouveau");
        let _ = (GTX_960, GTX_650, FERMI);
    }

    #[test]
    fn a_machine_with_two_cards_lists_both() {
        let lspci = format!("{INTEL}\n{RTX_3060}\n");
        let found = detect_gpus_from_lspci(&lspci);
        assert_eq!(found.iter().map(|g| g.vendor.as_str()).collect::<Vec<_>>(), ["Intel", "NVIDIA"]);
    }

    #[test]
    fn a_gtx_550_is_not_taken_for_a_turing_workstation_card() {
        // "t550" must not match inside "gtx550": that card is Fermi.
        assert!(!is_nvidia_turing_or_newer("GeForce GTX550 Ti"));
        assert!(is_nvidia_turing_or_newer("T550 Laptop GPU"));
    }
}

#[cfg(test)]
mod package_tests {
    use super::*;

    #[test]
    fn the_package_sets_are_names_the_apt_job_accepts() {
        // The page installs them through the apt job, which validates every name.
        let all = AMD_PACKAGES.iter().chain(&INTEL_PACKAGES).chain(&VMWARE_PACKAGES).chain(&QEMU_PACKAGES).chain(&VIRTMANAGER_PACKAGES).chain(&BOXES_PACKAGES).chain(&AMD_ROCM_PACKAGES).chain(&AMD_HIP_PACKAGES).chain(&AMD_MATH_PACKAGES).chain(&AMD_TOOLS_PACKAGES).chain(&INTEL_COMPUTE_PACKAGES).chain(&INTEL_MEDIA_PACKAGES).chain(&INTEL_TOOLS_PACKAGES);
        for name in all {
            assert!(crate::validate::apt_package(name), "{name}");
        }
    }
}
