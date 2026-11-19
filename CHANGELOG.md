# Changelog

All notable changes to Soplos Store are documented in this file.

## [3.0.0] - 2026-11-16

First stable release: the program is no longer in beta.

### Added

- An installed application has an Open button next to Uninstall, on its page: it starts the application as its launcher in the menu does (its desktop file for APT and AppImages, `flatpak run` and `snap run` for Flatpak and Snap). An application with nothing to open, such as a library or a tool of the terminal, has no Open button.

### Fixed

- The package had no section, so APT repositories managed with reprepro refused it ("No section given"). It is in the section `admin`, with the priority `optional`, as the other Soplos programs that change the system.
- The footer named the desktop as `XDG_CURRENT_DESKTOP` writes it ("XFCE", "KDE"); it now says Xfce, Plasma or GNOME, as the other Soplos 3.0 programs.
- A .deb file opened from the file manager had no Open button once installed: its page only said Installed. The button now shows as soon as the package is installed, and also when it was installed already. What it opens is read from dpkg itself, the launchers the package put in the menu (`dpkg-query -L`), so it works for any package, also one that is in no catalog.

## [3.0.1-5-beta] - 2026-11-14

### Changed

- Drivers, NVIDIA: installing a driver (repository of Debian and of NVIDIA) and removing it put `nvidia-drm.modeset=1` where the boot loader of the system reads it, and write its menu again: `/etc/default/grub` and `update-grub` with GRUB, `/etc/kernel/cmdline` and `limine-update` with Limine, `/etc/kernel/cmdline` and `kernel-install add` for every kernel with systemd-boot, the three that Soplos Linux Installer offers. The boot loader is detected as Soplos Welcome does it (`src-tauri/src/boot.rs`).
- The initramfs is rebuilt with `update-initramfs -u -k all` instead of `dracut --force`: on Soplos it is dracut's own wrapper, which also runs the hooks that copy the new initramfs to the EFI partition for systemd-boot, and it does every kernel, not only the running one.
- NVIDIA branches: 620 (Turing and newer) replaces 590 and 610; 580 stays for Maxwell and Pascal. The 620 branch is installed as NVIDIA publishes it for Debian 13, `nvidia-driver-pinning-620` and then `nvidia-open`. The helper refuses 590 and 610.

### Fixed

- The GTX 1650 and 1660 (Turing) were recommended nouveau: "gtx 16" was matched as a whole word, which a model number never is. They get the 620 branch.
- The NVIDIA 590 and 610 branches installed `nvidia-open-590` and `nvidia-open-610`, packages that NVIDIA's Debian 13 repository does not have.

## [3.0.1-4-beta] - 2026-11-12

### Fixed

- Installed apps: removing LibreOffice did nothing when the metapackage `libreoffice` had already been removed. The entry stood for that one package, so `apt-get remove libreoffice` had nothing to remove, finished with no error and the page took the app as removed, while `libreoffice-writer`, `libreoffice-core`, `libreoffice-common` and the rest stayed installed and the entry came back when the list was read again. An entry that is a suite of many packages now carries all of them, and removing it removes every one that is installed, also when several entries are selected and removed together.

## [3.0.1-3-beta] - 2026-11-04

### Added

- New page in Drivers, 2-in-1 laptops: it installs `tablet-soplos`, which every Soplos edition has in its own repository with the screen rotation by the accelerometer and the on-screen keyboard that fits its desktop.

### Changed

- The manual page describes the Drivers section, the sources and the theme in Settings, and the Updates page, and lists the new 2-in-1 laptops page.
- New screenshot of the Updates page when everything is up to date.

### Fixed

- The context menu of the web engine (Back, Forward, Reload, Inspect) no longer opens on a right click in the installed program. It stays available while the program is run for development.
- The application metadata points to the launcher that the package installs, `soplos-store.desktop`.

## [3.0.1-2-beta] - 2026-10-29

### Changed

- The window no longer has the frame of the system: it is a card with rounded corners and its own minimize, maximize and close buttons at the top right. It can be moved from the top strip, the banner and the side bar, and double click on the strip maximizes it. The rounded corners need a compositing manager running, as the one of Xfce, Plasma or GNOME.

## [3.0.1-1-beta] - 2026-10-28

### Changed

- New application icon, in the sizes of the package, the window and the interface.
- New hero image, with the circuit style of the other Soplos 3.0 programs.
- New screenshots of Explore, App page, Categories, Category page, Installed, Batch uninstall and Settings, taken with the interface in English.

## [3.0.1-beta] - 2026-10-28

### Added

- Settings: a switch to show each source in the Store (Flatpak, Snap and AppImage), on by default. With one off, the Store shows nothing of that source (catalog, installed apps and updates), does not look for its updates and does not count them in the tray, for whoever means to use another way to install and update it.
- Settings: a switch for the Drivers section of the sidebar, off by default until Drivers is finished. The app goes back to Explore if it is turned off while Drivers is open.
- Settings: a "Home page" block with a switch for each of the Featured, Popular and Trending sections. A section that is turned off disappears from Explore.
- Updates: two blocks over the list, "Removed" and "Added", for the packages that go out and come in with the updates, as GNOME Software shows them (a renamed library, for example, is replaced). They are only told: nothing is selected for them. They are kept in `~/.cache/soplos-store/updates.json` with the result of the check.
- Home page: the featured snaps of the Snap Store take part in Featured, and the snaps of the Snap Store categories that match each category of the Store appear in them, only with snapd installed. They come with their icon and first screenshot.

### Changed

- Updates: the list is what apt would upgrade, the way PackageKit and GNOME Software see it, which is the older solver of apt (`APT::Solver=internal`) and not the 3.0 one that the `apt` command uses. Updates that apt holds back because of their dependencies, such as `gnome-shell-extension-prefs`, are no longer listed or counted, so the tray notification does not come back for them. When apt cannot make that plan, the list is the one `apt list --upgradable` prints.
- The root helper no longer upgrades with `--no-remove`. It works out the plan of a full upgrade itself, simulates the upgrade that was asked for and refuses it if it would remove anything that plan does not remove, so a package that needs a renamed library replaced (`libavcodec-extra`) can be installed and nothing else can be taken out.
- Updates: the bar with "Check now" and "Update all" stays at the top while the list scrolls.
- Updates: each button has its own line along its lower edge. For APT it counts the steps of that package, its download (2) and the six steps of dpkg, read from the status lines apt writes with `APT::Status-Fd`, which are the same in every language. The bar across the whole row is gone. Where a tool does not say how far it is (Flatpak, Snap, several firmware devices at once) the button only shows the stage and has no line.
- Home page: Featured, Popular and Trending mix the lists of Flathub, only when Flatpak is installed, with rankings of Debian apps made with the ODRS ratings, which come from users of every distribution: Featured is the best rated with a minimum of 20 reviews, Popular the most reviewed and Trending the apps with a version released in the last 30 days. The banner of every section and category shares its slides out between APT, Flatpak and Snap, taking the apps with a screenshot of each in turn, and the first ones of each source take turns week by week. Before, the banners came almost all from Flathub.
- Without Flatpak, Flathub is not asked at all: not for the home page, the categories, the search or the app pages, which are made of what the other sources say. The same goes for Snap.
- Settings: the Recommendations block was replaced by the "Home page" one.
- The package now depends on `appstream`, which makes apt download the AppStream data of Debian that the catalog is built from, and on `packagekit`.

### Removed

- Settings: the switches "Featured and trending apps", "Ratings and reviews (ODRS)", "Show proprietary software", "Show all packages" and "Update Flatpak apps automatically". None of them did anything.

### Fixed

- The progress of APT updates never moved in languages other than English, because it looked for English words in the output of apt. It now reads the status lines, and those lines no longer show in the log.
- A package that apt could not install was listed anyway: updating `libavcodec-extra` did nothing and left it in the list, and the held back ones made the notification come back at every check.
- Flatpak install scope: Settings offered "User", "System" and "Ask every time" even when Flathub is set up in one installation only, and the install used that choice without looking. Now, with Flathub in only one of the two, Settings shows it as text and the app is installed there without asking, as Soplos Welcome 3.0 does; with Flathub in both, the choice in Settings is used, and "Ask every time" still asks.

## [3.0.0-9-beta] - 2026-10-24

### Fixed

- Settings and Open package file: they had a fixed maximum width, so a maximized window was left half empty. Settings now flows its blocks into two columns on a wide window (one column on a narrow one), and the page of a package file uses the whole width.

## [3.0.0-8-beta] - 2026-10-24

### Added

- Updates: the button of each app now says what is happening to it (Preparing, Downloading, Installing) and a thin bar under its row shows how far it is, both for the button of the app and for "Update all". It is read from the output of the job: for apt, the files downloaded and the packages already set up; for the other sources, the percentage the tool prints when there is one and the stage otherwise.

- Drivers, Laptops page: a new "Apple Mac" section with mbpfan (fan control for MacBooks, from Debian) and the FaceTime HD camera driver (facetimehd, a community project that is not in Debian). The camera driver is built into a local package from the project sources at a pinned commit, so DKMS rebuilds it for every kernel and the Store can remove it; the camera firmware is downloaded from Apple's update servers and checked against pinned checksums. It is marked as experimental, as the project itself says. Both were tested on MacBooks by a user of the community.

## [3.0.0-7-beta] - 2026-10-23

### Added

- The window follows the light or dark theme of the desktop. A new "Appearance" block in Settings lets the user choose Automatic (the default, which follows the desktop and notices when it changes), Light or Dark, and the button of the sidebar still switches the theme and keeps what it chose. The theme is read from the XDG portal and, when that says nothing, from each desktop: the colors of `kdeglobals` on Plasma, the name of the GTK theme on Xfce (a guess from words such as "dark") and the color scheme of GNOME. It is checked again every few seconds and when the window gets the focus.

### Changed

- The output of the jobs is one log for everything that ran since the strip was last closed, with a header for each job, instead of the log of the last one only: when an update went through APT and then Flatpak, the output of the first was lost. The log has a fixed height of 200 pixels, with its own scroll, instead of a maximum that made it grow with every line.
- Drivers: Epson, Brother and Canon point to their international pages: Epson to its global support site (the Epson Scan 2 page for scanners and the ESC/P-R 2 page for printers), Brother to its support site, which sends each visitor to their country, and Canon to its global support page.
- Drivers, graphics tablets: Wacom is one card, with its packages and a link under the description to its official page, instead of two. A card that installs can now carry that link.

## [3.0.0-6-beta] - 2026-10-23

### Added

- Drivers: the official page of each maker is in place on every page that has makers, and each was opened and checked: Epson (its Linux support page), Canon, Brother, Wacom, XP-Pen, Huion, Gaomon, Veikk, Ugee, Zebra, TSC, Blackmagic, Lenovo (its Linux page, which adapts to the country), Dell, HP, Framework, TUXEDO, CyberPower and Eaton. Those without a language in the address are shown in the language of the country, and the ones that were in Spanish only were changed to international addresses.
- Drivers: a link that goes to a project kept by the community, not by the maker, is marked as such on its card ("Community driver for ...", with a warning to check the project first). AVerMedia (the open `open_gc573` driver of EverLive for the GC573), Parblo (the Linux support for its Coast tablets) and ASUS (asus-linux, with asusctl and supergfxctl) are the first.
- Drivers, laptops: Slimbook, with its apps page. Wi-Fi and Bluetooth: Ralink, whose firmware is in `firmware-mediatek`. Pro audio: Scarlett, with `alsa-scarlett-gui` for Focusrite interfaces.
- Drivers: logos for the makers and tools of the new pages, from their own sites, Simple Icons or Wikimedia Commons, trimmed of margins; SANE, DIGImend and Steam Deck included. Those that are white for dark headers were redrawn in dark grey so that they show on the white tile of the card.

### Changed

- Drivers, printers: Ricoh points to its global support page (the old one was a firmware center from 2002), Epson to its Linux support page and Brother to its drivers page. Samsung points to the page Samsung keeps about printer support with HP, and Canon to its support page.

### Removed

- Drivers: the makers that have no Linux driver and no page about it are gone from the pages: Elgato, Fujitsu (its scanners work with SANE), Focusrite, Behringer, PreSonus and APC (the apcupsd card covers it).

## [3.0.0-5-beta] - 2026-10-22

### Added

- Drivers, Printers section. The hero starts on the printers plugged in by USB (found in sysfs, with no tool and no CUPS needed, and told apart by maker) and goes on through HP, Brother, Epson, Canon, Xerox, Ricoh, Kyocera, Lexmark, OKI, Konica Minolta, Samsung and Pantum with their official logos. Below it: the printing system (CUPS with its filters, driverless USB printing and the printer settings tool, a print-to-PDF printer, Gutenprint), a section for each maker that has packages in Debian (HP with HPLIP, Brother with brlaser, Epson with ESC/P-R, Xerox, Ricoh, OKI and Samsung with their drivers) and one for the makers that only publish their drivers per model on their own website (Canon, Kyocera, Lexmark, Konica Minolta, Pantum), where a card opens the official downloads page. Every maker with packages also has that card. Label and receipt printers go in their own category, not here. Network printers are not detected yet.

- Drivers: the other thirteen categories now have their page, made from a description in `src/lib/driverPages.ts` that one view (`DriversPage`) draws: scanners, graphics tablets, labels and tickets, capture cards, gaming peripherals, Wi-Fi and Bluetooth, laptops, pro audio, network, fingerprint readers and security, UPS and RGB and cooling, and virtual machines. Each has the hero going through every card and, below it, a few sections: the tools with the Debian packages each one installs and, grouped in one section, the makers that only publish their drivers on their own page (their card says it is coming until each page is found). Every card carries the official logo of its maker or tool, and an icon when there is none. RGB and cooling installs OpenRGB with its udev rules and the `i2c-dev` module through the fixed actions of the root helper. A new command, `get_package_sets`, answers whether a list of packages is installed for any page, refusing names that are not package names.

### Changed

- The Categories page uses the same cards as the Drivers page: a dark showcase with the color of the category glowing behind its icon, the name and a line with some of its subcategories. Both pages share one component.

### Fixed

- The updates counted in the background, and so the notification and the list the Updates page showed, came from package lists that were not up to date, and only "Check now" (which runs `apt-get update`) found the rest. The tray never refreshed the lists before checking, and the timer did it only when they were older than a whole check interval, which skipped the refresh about every other time because the lists the previous check had refreshed were always a little younger than that. Both now go through one function that refreshes the lists when they are older than half the interval ("Check now" of the tray menu always does), and a refresh that fails is written to the log instead of being ignored.
- Opening the Updates page, from the menu, the notification or the tray, showed the stored result even when it came from old lists, so some updates only appeared after pressing "Check now". The page now looks for real every time it opens: it refreshes the package lists and asks every source, whatever the stored result or the age of the lists, so updates made meanwhile in a terminal are seen, and it says it is searching until it has the answer.

## [3.0.0-4-beta] - 2026-10-22

### Added

- The Drivers page opens on the cards of its categories (graphics, printers, scanners, graphics tablets, labels, capture cards, gaming peripherals, Wi-Fi and Bluetooth, laptops, pro audio, network, fingerprint readers, UPS, RGB and cooling, virtual machines); only graphics is ready, the rest say they are coming. Choosing Drivers again while inside a section goes back to the categories. Graphics has a section per brand, with no one made to look like the main one.
- Graphics, more complete: AMD gets the ROCm runtime and OpenCL, HIP development, the math libraries and monitoring tools (Debian packages); Intel gets OpenCL and Level Zero, oneVPL, monitoring tools and the oneAPI stacks (DPC++ compiler, Math Kernel Library, VTune and the Base Toolkit) from the official Intel repository; NVIDIA gets the CUDA Toolkit from the official NVIDIA repository. The stacks of the makers are installed by a fixed action of the root helper (a closed list, the repository is added, used and removed again) and ask for a confirmation first.
- A new Drivers page, first section: graphics. Its hero starts on your own graphics card (vendor, model, kind, state of its driver and the action that fits it) and then moves on by itself through NVIDIA, AMD, Intel, VMware, VirtualBox, QEMU/KVM, Virt-Manager, GNOME Boxes and Proxmox, each with its official logo; the drivers or tools of the brand on screen appear below (VMware, QEMU and Proxmox guest tools, Virt-Manager and GNOME Boxes install from Debian; VirtualBox only explains where its Guest Additions come from, since Debian has no package for them). The carousel stops while the pointer is over the page or something is being installed. For NVIDIA, the cards below are the driver branches, with the brand colors: 610, 590 and 580 from NVIDIA's CUDA repository, 550 from Debian, and the legacy 470, 390 and 340 (these need Debian Sid). The branch that fits the card is marked as recommended, the 610 and 590 are blocked on cards older than Turing, and installing or removing asks once inside the card. AMD and Intel cards get an "Install the missing drivers" button for their Mesa and firmware packages. It ported the detection and the installers from Soplos Welcome 3.0, kernel patches included (Fix 5, 6 and 7), but the scripts are not written to a file and run from there as in Welcome: the helper that runs as root builds them from a branch name checked against a closed list, so the Store keeps accepting only fixed actions.
- AppImage updates: the Store now looks for new versions of the AppImages that have a version and an update address saved in Soplos AppImage Manager (`X-AppImage-Version` and `X-AppImage-UpdateUrl` in their launcher), with the same logic as its "Search for updates": the latest release of a GitHub, GitLab or Codeberg project, or the date of a direct `.AppImage` link. They show in the updates list, in "Check now" and in the tray count. Updating downloads the new file, replaces the old one and rewrites the launcher with the new version and path, so AppImage Manager sees the AppImage as updated. The version and the address are still filled in by hand in AppImage Manager; the Store only reads them. AppImages without a version or an address are skipped, and the check log says how many and where to fill them in. Unlike AppImage Manager, a direct link whose server gives no date is not offered, because it would be listed as an update for ever.
- Settings: a "Firmware updates" switch in the Firmware row. With it off, firmware is left out of the update list, of "Check now" (which no longer runs `fwupdmgr refresh`) and of the count the tray notifier shows, so an unreachable LVFS cannot hold the checks up. It is on by default. When fwupd is not installed the row still shows the "Install fwupd" button, with the switch visible but disabled until it is. The firmware testing channel is a switch too, instead of an Enable/Disable button; it follows what fwupd reports, so it goes back if the change fails.
- `CHANGELOG.md` is installed as `/usr/share/doc/soplos-store/changelog.gz`.
- AppStream metainfo: categories and keywords in the eight languages, the same ones as the desktop entry, and the `3.0.0-alpha` release. The note of `3.0.0-beta` no longer says it is the first release.
- The Debian copyright now declares Lucide (ISC), used for the interface icons.
- README: version, license, platform, Tauri, Rust and Svelte badges, an Installation section, and a Documentation section with the manual page and the command line options.

### Changed

- The Updates page and the tray notifier now share one stored result. A check looks in every source once, all at the same time, and keeps the whole list (not only the count) in `~/.cache/soplos-store/updates.json`. The notification counts that list and the page shows it, so the two numbers always agree. Opening the page, from the notification or from the menu, shows the stored list at once: it no longer runs `apt update` or looks in the sources again. "Check now" is the only thing that does that work, and it is a single job: refresh the package lists, refresh the firmware metadata (without `--force`) and look in every source. After updating packages the list is read again from the sources, without `apt update`.
- The Updates page is one block, with no separate header: the left menu already names it. It shows the spinner while it searches, the "everything is up to date" message (with the date of the last check and the "Check now" button under it), or the list. In the list, one bar holds the selection with the number of updates, "Check now" and "Update all", and the last check, package lists and sources are written once at its foot. Before, "Searching" and "Everything is up to date" were written in a header and again in the middle, under the same title as the menu.
- The log of "Check now" gives one block per source, with the updates found, their count and how long that source took to answer. Debian's app metadata is read again in the background after the refresh, and the page no longer waits for it.
- The cards and the featured carousel no longer have an Install button: they only open the app page, where the description, the screenshots and the source are, and the app is installed from there. The app page starts on the source the app is installed from, or on APT.
- The "Preferred source" setting is gone: with the source chosen on each app page it had no use left, and its "Ask on each app" option never did anything. The sources are listed in the catalog order, APT first.
- Manual page: new keyboard shortcuts section. It is installed compressed, as `/usr/share/man/man1/soplos-store.1.gz`.
- README introduction now mentions AppImages and firmware updates through fwupd, and the version badge is back in step with the package.

### Fixed

- Opening the Updates page from the tray notification looked for updates again from zero, which took far longer than the notification itself, and could show another number than the one the notification gave (it refreshed the package lists first). Each source was also asked twice in every check, once for the log and once for the page, and Debian's app metadata was rebuilt each time.
- "Check now" and turning on the firmware testing channel could hang for good when the LVFS was down: `fwupdmgr refresh --force` waited for the server with no time limit. It now runs through `timeout` and is stopped after two minutes. Installing a firmware update (`fwupdmgr update`) has a limit too, of 30 minutes because the download and the writing to the device take time, and when a step is stopped by its limit the log says so instead of ending without a word.
- The firmware testing channel row was hidden when fwupd was not installed. It is now always shown, disabled while fwupd is missing or firmware updates are turned off.
- AppStream metainfo: the developer and project group identifiers were `org.soploslinux`. They are now `org.soplos`, matching the soplos.org domain and the other Soplos apps.
- AppStream metainfo: the screenshots were bare file names instead of web addresses, so software centers, this one included, could not show them and the file did not validate. They now use their `raw.githubusercontent.com` address.
- The hourly update check option (`settings.every1`) had no text in German, French, Italian, Portuguese, Romanian and Russian.
- `sections.topIn` was left in English in German and now reads "Das Beste in {section}", like the other languages.

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
