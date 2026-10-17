// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// WebKitGTK's DMA-BUF renderer leaves the window blank on virtual GPUs
/// (VMware, VirtualBox, QEMU/KVM) without reporting any error, so it is
/// turned off before the webview starts when a virtual machine is detected.
fn running_in_vm() -> bool {
    const MARKERS: [&str; 6] = ["vmware", "virtualbox", "innotek", "qemu", "kvm", "bochs"];
    ["/sys/class/dmi/id/sys_vendor", "/sys/class/dmi/id/product_name", "/sys/class/dmi/id/board_vendor"]
        .iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .any(|value| {
            let value = value.to_lowercase();
            MARKERS.iter().any(|marker| value.contains(marker))
        })
}

fn main() {
    // Started by pkexec for an APT operation: do it and exit, no window.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--privileged") {
        std::process::exit(soplos_store_lib::run_privileged(&args[2..]));
    }
    // Update notifier started with the session: tray icon only, no window.
    if args.get(1).map(String::as_str) == Some("--tray") {
        std::process::exit(soplos_store_lib::run_tray());
    }
    // Update check run by the systemd user timer: no window.
    if args.get(1).map(String::as_str) == Some("--check-updates") {
        std::process::exit(soplos_store_lib::run_check());
    }
    if running_in_vm() && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    soplos_store_lib::run()
}
