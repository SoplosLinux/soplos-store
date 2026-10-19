# Changelog

All notable changes to Soplos Store are documented in this file.

## [3.0.0-3-beta] - 2026-10-19

### Added

- Real-time synchronization between Store Settings and the systemd user timer (`soplos-store-updates.timer`), creating dynamic drop-in overrides (`override.conf`) and enabling/disabling the timer according to user choices.
- Added hourly ("1h") update check frequency option across settings UI, translations, and backend timer integration.
- Added `"testing-on"` and `"testing-off"` actions to `JobAction` type in TypeScript to properly type LVFS testing channel toggles.

### Changed

- Updates tab triggers fresh update check unconditionally on mount, eliminating timer-based age restrictions when the user deliberately opens the updates view.
- Unified Updates tab layout: stable permanent top header containing status and action buttons, completely eliminating duplicated buttons and centered layout shifts.
- Fixed `checker.rs` to rely on the systemd timer interval rather than discarding checks with a redundant local age filter.

## [3.0.0-2-beta] - 2026-10-19

### Added

- Manual page documentation (`docs/soplos-store.1`) installed to `/usr/share/man/man1/` covering application usage, options, supported file formats, and system files.
- Full 8-language internationalization across desktop application launchers (`soplos-store.desktop.hbs`), tray autostart entries (`soplos-store-tray.desktop`), and AppStream metadata (`org.soplos.store.metainfo.xml`).
- Complete 9-screenshot application showcase covering Explore, App Detail, Categories, Category Detail, Installed, Batch Selection, Updates, Up-to-Date view, and Settings.

### Changed

- Redesigned Updates view with clean per-state layouts: single unified centered view during refresh and up-to-date states, and toolbar action header with package list when updates are available.
- Streamlined batch uninstall in Installed tab: activation via dedicated "Select" button, full-card tap selection, and removal of hazardous "Select all" control.
- Stabilized toolbar in Installed tab with non-wrapping controls to eliminate visual jumping when selecting applications.

## [3.0.0-1-beta] - 2026-10-18

### Added

- Multi-selection checkboxes and batch uninstallation in "Installed" tab (`Installed.svelte`), allowing bulk removal across mixed sources (APT, Flatpak, Snap, AppImages).
- Full AppImage integration (`appimage.rs`): scans installed AppImages from `~/AppImages` with their desktop launchers, and provides direct "Install" support when opening `.AppImage` package files, automatically copying to `~/AppImages`, granting executable permissions, and generating desktop launchers.
- Suite unification for LibreOffice in "Installed" tab: consolidates individual packages (`libreoffice-base`, `libreoffice-calc`, `libreoffice-math`, `libreoffice-writer`, etc.) under a single, unified "LibreOffice" suite entry.
- Robust `.desktop` file fallback discovery for installed applications (`desktop.rs`), ensuring GUI apps (e.g. LibreOffice, KMail, GIMP) are always detected in "Installed" even when DEP-11 AppStream metadata is incomplete or absent.

### Changed

- Separated APT repository refresh into a dedicated Polkit action with `<allow_active>yes</allow_active>` via `--privileged-refresh`, allowing update checks without password prompts.
- Reduced the update check timer startup delay from 3 minutes to 1 minute (`OnStartupSec=1min`).
- Updated tray icon to use the standard `software-update-available` status icon while keeping the official `soplos-store` brand icon for desktop notifications.
- Fixed notification repetition logic in `tray.rs` so pending updates notify reliably on startup and at configured intervals.
- Added desktop entry hints and graceful fallback for notifications across all supported desktops.
- Auto-refresh stale APT lists on Updates tab mount without password dialogs.
- Unified Updates tab layout with a permanent, stable header, integrated "Select all" control, and consistent states for loading, up-to-date, and available updates without jarring layout shifts.
- Dynamically display only available/installed package sources in the Updates tab (hiding Snap or Flatpak if not present on the system).

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
