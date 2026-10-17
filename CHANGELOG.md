# Changelog

All notable changes to Soplos Store are documented in this file.

## [3.0.0-beta] - 2026-10-17

### Added

- Firmware updates through fwupd: a "Firmware" row in Updates, included in the check and the tray count. Settings shows fwupd (with install if missing) and can enable the LVFS testing channel. `fwupd` is now a package dependency.
- Subcategories in every category page, from the AppStream categories of Flathub and Debian, in a drop-down with the number of apps.
- A fuller app page: screenshot gallery, ratings histogram, details and project links.
- Update checks run from a systemd user timer (`soplos-store-updates.timer`) that stores the result; the tray only shows it and the app starts it when opened.
- The package refreshes the APT package lists daily through Debian's own timer (`APT::Periodic::Update-Package-Lists`).
- Banners with five slides, a collapsible search and cards of apps below the featured banner.

### Changed

- Renamed from Soplos Software Center to Soplos Store.

## [3.0.0-alpha] - 2026-10-16

First release of Soplos Store, built with Tauri 2, a Rust backend and a Svelte frontend.

### Added

- Explore: a carousel of highlighted apps for each section (featured, popular, trending and every category), with the app's screenshot as background and a new app every few seconds, plus the other apps of the section below it.
- Catalog from public databases: Debian's AppStream metadata (DEP-11, read from the lists apt already downloads), the Flathub API and the Snap Store, with ratings from ODRS. An app available from several sources appears once, with a source selector on its page.
- Search across Debian, Flathub and Snap at once.
- Categories, each with its own page: a carousel with the best apps, a source filter and sorting.
- App page: icon, screenshot, source selector with version, description, details and screenshots. Texts follow the system language when Debian has a translation.
- Installed (APT, Flatpak and Snap) and Updates, with updates installed one by one or all at once.
- Install, uninstall and update for APT, Flatpak and Snap. APT operations run through pkexec with a polkit policy of their own, limited to a fixed set of actions with validated package names. Flatpak installs for the user or the whole system, as chosen in Settings. Progress and output are shown live, with the full log on demand.
- Package files: .deb, .flatpak, .flatpakref and .snap open in this app and can be installed from it; .AppImage files are handed to Soplos AppImage Manager.
- Update notifier (`--tray`), started with the session: a tray icon and a desktop notification while updates are available. Clicking either opens the app on Updates.
- Settings: software sources (APT, Flatpak, Snap, AppImage), install scope, preferred source, update checks, recommendations and maintenance (orphans, cache, unused Flatpak runtimes). Opens Soplos Repo Selector and Soplos AppImage Manager, and offers to install them when missing.
- Keyboard shortcuts as in the other Soplos 3.0 apps: F1 (About), Ctrl+Q, Ctrl+Tab and Ctrl+Shift+Tab, plus F5 or Ctrl+R to reload, Ctrl+F to search and Escape or Alt+Left to go back.
- Interface in English, Spanish, German, French, Italian, Portuguese, Romanian and Russian, with light and dark themes.
- Flatpak and Snap are only offered as sources when they are installed.
