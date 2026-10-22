// The developer stacks that only their maker publishes: the oneAPI tools of
// Intel and the CUDA Toolkit of NVIDIA. Like the NVIDIA drivers of nvidia.rs
// they run as scripts built into this binary: the caller only names one of
// the ids below, the repository is added, the packages are installed and the
// repository goes away again, so the system does not keep a third-party
// source around.

/// Every id the helper accepts, with the packages it installs.
pub const ITEMS: [(&str, &[&str]); 6] = [
    ("oneapi-dpcpp", &["intel-oneapi-compiler-dpcpp-cpp"]),
    ("oneapi-mkl", &["intel-oneapi-mkl-devel"]),
    ("oneapi-vtune", &["intel-oneapi-vtune"]),
    ("oneapi-base", &["intel-oneapi-base-toolkit"]),
    ("cuda-toolkit", &["cuda-toolkit"]),
    ("openrgb", &["openrgb"]),
];

/// The packages of an id, if it is in the list.
pub fn packages(id: &str) -> Option<&'static [&'static str]> {
    ITEMS.iter().find(|(name, _)| *name == id).map(|(_, packages)| *packages)
}

pub fn is_item(id: &str) -> bool {
    packages(id).is_some()
}

/// The script that installs one item from the repository of its maker.
pub fn install_script(id: &str) -> Result<String, String> {
    let packages = packages(id).ok_or_else(|| format!("Not a developer stack of the Store: {id}"))?.join(" ");
    let script = if id == "openrgb" {
        // The Debian package ships no udev rules, and OpenRGB 1.0 generates
        // them itself, without a display. The i2c-dev module is loaded for
        // the motherboard and memory controllers.
        format!(
            r#"export DEBIAN_FRONTEND=noninteractive
echo "Installing OpenRGB..."
apt-get install -y {packages}
echo "Installing the udev rules..."
mkdir -p /etc/udev/rules.d
openrgb --print-udev-rules > /etc/udev/rules.d/60-openrgb.rules.new
if [ ! -s /etc/udev/rules.d/60-openrgb.rules.new ]; then
    rm -f /etc/udev/rules.d/60-openrgb.rules.new
    echo "ERROR: OpenRGB did not generate its udev rules."
    exit 1
fi
mv /etc/udev/rules.d/60-openrgb.rules.new /etc/udev/rules.d/60-openrgb.rules
echo i2c-dev > /etc/modules-load.d/soplos-openrgb.conf
modprobe i2c-dev || true
udevadm control --reload-rules
udevadm trigger
echo "OpenRGB installed with its udev rules."
"#
        )
    } else if id == "cuda-toolkit" {
        format!(
            r#"set -e
echo "=== CUDA Toolkit from the official NVIDIA repository ==="
TEMP_DIR=$(mktemp -d)
wget -q -O "$TEMP_DIR/cuda-keyring.deb" \
    https://developer.download.nvidia.com/compute/cuda/repos/debian13/x86_64/cuda-keyring_1.1-1_all.deb
if [ ! -s "$TEMP_DIR/cuda-keyring.deb" ]; then
    echo "ERROR: Failed to download cuda-keyring."
    rm -rf "$TEMP_DIR"
    exit 1
fi
dpkg -i "$TEMP_DIR/cuda-keyring.deb"
rm -rf "$TEMP_DIR"
apt-get update -q
apt-get install -y --no-remove {packages}
apt-get purge -y cuda-keyring
rm -f /etc/apt/sources.list.d/cuda-debian13-x86_64.list
apt-get update -q
"#
        )
    } else {
        format!(
            r#"set -e
echo "=== Intel oneAPI from the official Intel repository ==="
KEY=/usr/share/keyrings/oneapi-archive-keyring.asc
LIST=/etc/apt/sources.list.d/oneAPI.list
trap 'rm -f "$LIST" "$KEY"' EXIT
wget -q -O "$KEY" https://apt.repos.intel.com/intel-gpg-keys/GPG-PUB-KEY-INTEL-SW-PRODUCTS.PUB
if [ ! -s "$KEY" ]; then
    echo "ERROR: Failed to download the Intel signing key."
    exit 1
fi
echo "deb [signed-by=$KEY] https://apt.repos.intel.com/oneapi all main" > "$LIST"
apt-get update -q
apt-get install -y --no-remove {packages}
trap - EXIT
rm -f "$LIST" "$KEY"
apt-get update -q
"#
        )
    };
    Ok(script)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn every_script_is_valid_shell_and_names_its_packages() {
        for (id, packages) in ITEMS {
            let script = install_script(id).unwrap();
            let file = std::env::temp_dir().join(format!("store-vendor-{id}.sh"));
            std::fs::write(&file, &script).unwrap();
            let out = Command::new("bash").arg("-n").arg(&file).output().unwrap();
            let _ = std::fs::remove_file(&file);
            assert!(out.status.success(), "{id}: {}", String::from_utf8_lossy(&out.stderr));
            for package in packages {
                assert!(script.contains(package), "{id} installs {package}");
                assert!(crate::validate::apt_package(package), "{package} is an apt name");
            }
        }
    }

    #[test]
    fn only_the_listed_ids_are_accepted() {
        assert!(install_script("oneapi-dpcpp; reboot").is_err());
        assert!(install_script("bash").is_err());
        assert!(install_script("").is_err());
        assert!(is_item("cuda-toolkit"));
    }
}
