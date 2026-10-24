// The developer stacks that only their maker publishes: the oneAPI tools of
// Intel and the CUDA Toolkit of NVIDIA. Like the NVIDIA drivers of nvidia.rs
// they run as scripts built into this binary: the caller only names one of
// the ids below, the repository is added, the packages are installed and the
// repository goes away again, so the system does not keep a third-party
// source around.

/// Every id the helper accepts, with the packages it installs.
pub const ITEMS: [(&str, &[&str]); 7] = [
    ("oneapi-dpcpp", &["intel-oneapi-compiler-dpcpp-cpp"]),
    ("oneapi-mkl", &["intel-oneapi-mkl-devel"]),
    ("oneapi-vtune", &["intel-oneapi-vtune"]),
    ("oneapi-base", &["intel-oneapi-base-toolkit"]),
    ("cuda-toolkit", &["cuda-toolkit"]),
    ("openrgb", &["openrgb"]),
    ("facetimehd", &["facetimehd-dkms"]),
];

// facetimehd is not in Debian. Both projects are fetched at the commit named
// here, never at a branch, so the root script always runs the code that was
// reviewed; the firmware script also checks every file it extracts against
// pinned checksums.
const FACETIMEHD_COMMIT: &str = "6dc8cd85e87993ff21cb01eb333ff42afb058551";
const FACETIMEHD_FIRMWARE_COMMIT: &str = "60ee21228d9ca00a7bd84fdaefaff00a81f1db91";

// Builds a local package from the driver sources so that dpkg owns the module
// (it shows as installed and apt can remove it); DKMS rebuilds it for every
// new kernel. The firmware comes from Apple's own update servers, extracted
// on this computer by the firmware project's script, and is removed with the
// package.
const FACETIMEHD_SCRIPT: &str = r##"set -e
export DEBIAN_FRONTEND=noninteractive
echo "=== FaceTime HD camera (facetimehd, community driver) ==="
apt-get install -y dkms build-essential git curl xz-utils ca-certificates
apt-get install -y linux-headers-$(uname -r) || true
if [ ! -e /lib/modules/$(uname -r)/build ]; then
    echo "Kernel build tree missing or broken, reinstalling headers..."
    apt-get install -y --reinstall linux-headers-$(uname -r) || true
fi

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

fetch_commit() {
    mkdir -p "$WORK/$1"
    git -C "$WORK/$1" init -q
    git -C "$WORK/$1" fetch -q --depth 1 "$2" "$3"
    git -C "$WORK/$1" checkout -q FETCH_HEAD
    if [ "$(git -C "$WORK/$1" rev-parse HEAD)" != "$3" ]; then
        echo "ERROR: $1 is not the commit that was expected."
        exit 1
    fi
}

echo "Downloading the driver sources..."
fetch_commit driver https://github.com/patjak/facetimehd @FACETIMEHD_COMMIT@
fetch_commit firmware https://github.com/patjak/facetimehd-firmware @FIRMWARE_COMMIT@

echo "Downloading the camera firmware from Apple and checking it..."
bash "$WORK/firmware/facetimehd-firmware-install.sh"
if [ ! -s /usr/lib/firmware/facetimehd/firmware.bin ] && [ ! -s /lib/firmware/facetimehd/firmware.bin ]; then
    echo "ERROR: the camera firmware was not installed."
    exit 1
fi

VERSION=$(sed -n 's/^PACKAGE_VERSION=//p' "$WORK/driver/dkms.conf" | tr -d '"')
if [ -z "$VERSION" ]; then
    echo "ERROR: could not read the driver version."
    exit 1
fi
PKG="$WORK/pkg"
SRC="$PKG/usr/src/facetimehd-$VERSION"
mkdir -p "$PKG/DEBIAN" "$SRC" "$PKG/usr/lib/modprobe.d"
cp -a "$WORK/driver/." "$SRC/"
rm -rf "$SRC/.git"
printf 'blacklist bdc_pci\n' > "$PKG/usr/lib/modprobe.d/facetimehd.conf"

cat > "$PKG/DEBIAN/control" <<CONTROL
Package: facetimehd-dkms
Version: $VERSION+git@SHORT_COMMIT@
Section: kernel
Priority: optional
Architecture: all
Depends: dkms
Description: FaceTime HD (Broadcom 1570) camera driver of the MacBook, DKMS source
 Community driver of the PCIe webcam of recent MacBooks, built by the Soplos
 Store from https://github.com/patjak/facetimehd. The firmware of the camera
 is downloaded from Apple at install time and removed with this package.
CONTROL

cat > "$PKG/DEBIAN/postinst" <<POSTINST
#!/bin/sh
set -e
if [ "\$1" = configure ]; then
    dkms add -m facetimehd -v $VERSION > /dev/null 2>&1 || true
fi
POSTINST
cat > "$PKG/DEBIAN/prerm" <<PRERM
#!/bin/sh
set -e
if [ "\$1" = remove ]; then
    dkms remove -m facetimehd -v $VERSION --all > /dev/null 2>&1 || true
fi
PRERM
cat > "$PKG/DEBIAN/postrm" <<POSTRM
#!/bin/sh
set -e
if [ "\$1" = remove ] || [ "\$1" = purge ]; then
    rm -rf /usr/lib/firmware/facetimehd /lib/firmware/facetimehd
fi
POSTRM
chmod 755 "$PKG/DEBIAN/postinst" "$PKG/DEBIAN/prerm" "$PKG/DEBIAN/postrm"

dpkg-deb --root-owner-group --build "$PKG" "$WORK/facetimehd-dkms.deb"
apt-get install -y --no-remove "$WORK/facetimehd-dkms.deb"

echo "Building the module for the running kernel..."
if ! dkms install -m facetimehd -v "$VERSION" -k "$(uname -r)"; then
    echo "ERROR: the module did not build for $(uname -r)."
    apt-get remove -y facetimehd-dkms || true
    exit 1
fi
modprobe -r bdc_pci 2> /dev/null || true
modprobe facetimehd || echo "The module could not be loaded now; restart the computer."
if command -v mokutil > /dev/null 2>&1 && mokutil --sb-state 2> /dev/null | grep -qi "SecureBoot enabled"; then
    echo "Secure Boot is on: the module is signed with the DKMS key, which has to be enrolled for the camera to load."
fi
echo "FaceTime HD camera driver installed."
"##;

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
    let script = if id == "facetimehd" {
        FACETIMEHD_SCRIPT
            .replace("@FACETIMEHD_COMMIT@", FACETIMEHD_COMMIT)
            .replace("@FIRMWARE_COMMIT@", FACETIMEHD_FIRMWARE_COMMIT)
            .replace("@SHORT_COMMIT@", &FACETIMEHD_COMMIT[..7])
    } else if id == "openrgb" {
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
