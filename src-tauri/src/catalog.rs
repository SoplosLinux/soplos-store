// Joins APT, Flatpak and Snap into one catalog: one card per app, listing
// every source it can be installed from. All commands run on the
// blocking thread pool so the window never freezes while loading.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde_json::Value;

use crate::apt::{self, AptApp};
use crate::flathub::{self, Hit};
use crate::models::{AppCard, AppDetail, AptStatus, InstalledApp, Section, SourceOption, SourcesStatus, UpdateItem};
use crate::{checker, flatpak, fwupd, odrs, snap, system};

/// Flathub category IDs and the matching AppStream main category.
pub const CATEGORIES: [(&str, &str); 10] = [
    ("audiovideo", "AudioVideo"),
    ("development", "Development"),
    ("education", "Education"),
    ("game", "Game"),
    ("graphics", "Graphics"),
    ("network", "Network"),
    ("office", "Office"),
    ("science", "Science"),
    ("system", "System"),
    ("utility", "Utility"),
];

struct InstalledState {
    apt: Arc<HashMap<String, (String, u64)>>,
    flatpak: HashSet<String>,
    snap: HashSet<String>,
}

fn installed_state() -> InstalledState {
    InstalledState {
        apt: apt::installed_packages(),
        flatpak: flatpak::installed_ids(),
        snap: snap::installed().into_iter().map(|s| s.name).collect(),
    }
}

fn normalize_name(name: &str) -> String {
    name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Card for an app found on Flathub. Flatpak is only offered as a source
/// when flatpak is installed; without it the app is shown only if APT has
/// it too, and None is returned otherwise.
fn card_from_hit(hit: &Hit, state: &InstalledState) -> Option<AppCard> {
    let apt_app = apt::find(&hit.id, &hit.name);
    // APT first: it is the system's own source. The frontend reorders
    // them by the preferred source chosen in Settings.
    let mut refs = Vec::new();
    if let Some(app) = apt_app {
        refs.push(apt_ref(app, state));
    }
    if flatpak::available() {
        refs.push(SourceOption { kind: "flatpak".into(), id: hit.id.clone(), installed: state.flatpak.contains(&hit.id), ..Default::default() });
    }
    if refs.is_empty() {
        return None;
    }
    let (rating, reviews) = best_rating(&hit.id, apt_app);
    Some(AppCard {
        id: hit.id.clone(),
        name: hit.name.clone(),
        summary: hit.summary.clone(),
        icon: hit.icon.clone(),
        screenshot: None,
        sources: refs.iter().map(|r| r.kind.clone()).collect(),
        installed: refs.iter().any(|r| r.installed),
        refs,
        rating,
        reviews,
        categories: {
            let mut all = hit.categories.clone();
            if let Some(app) = apt_app {
                all.extend(app.categories.iter().filter(|c| !all.contains(c)).cloned().collect::<Vec<_>>());
            }
            all
        },
    })
}

/// ODRS may know an app under its Flathub ID, its Debian ID or its
/// desktop file name; the entry with the most ratings wins.
fn best_full_rating(id: &str, apt_app: Option<&AptApp>) -> Option<odrs::Rating> {
    let mut keys = vec![id.to_string()];
    if let Some(app) = apt_app {
        keys.push(app.id.clone());
        keys.extend(app.desktop_ids.iter().cloned());
    }
    keys.iter().filter_map(|key| odrs::rating(key)).max_by_key(|found| found.count)
}

fn best_rating(id: &str, apt_app: Option<&AptApp>) -> (Option<f32>, u32) {
    match best_full_rating(id, apt_app) {
        Some(found) => (Some(found.average), found.count),
        None => (None, 0),
    }
}

fn apt_ref(app: &AptApp, state: &InstalledState) -> SourceOption {
    SourceOption { kind: "apt".into(), id: app.package.clone(), installed: state.apt.contains_key(&app.package), ..Default::default() }
}

fn snap_ref(name: &str, state: &InstalledState) -> SourceOption {
    SourceOption { kind: "snap".into(), id: name.to_string(), installed: state.snap.contains(name), ..Default::default() }
}

fn card_from_apt(app: &AptApp, state: &InstalledState) -> AppCard {
    let (rating, reviews) = best_rating(&app.id, Some(app));
    let reference = apt_ref(app, state);
    AppCard {
        id: app.id.clone(),
        name: app.name.clone(),
        summary: app.summary.clone(),
        icon: app.icon.clone(),
        screenshot: app.screenshots.first().cloned(),
        sources: vec!["apt".into()],
        installed: reference.installed,
        refs: vec![reference],
        rating,
        reviews,
        categories: app.categories.clone(),
    }
}

fn card_from_snap(found: &snap::StoreSnap, state: &InstalledState) -> AppCard {
    let reference = snap_ref(&found.name, state);
    AppCard {
        id: format!("snap:{}", found.name),
        name: found.title.clone(),
        summary: found.summary.clone(),
        icon: found.icon.clone(),
        screenshot: None,
        sources: vec!["snap".into()],
        installed: reference.installed,
        refs: vec![reference],
        rating: None,
        reviews: 0,
        categories: Vec::new(),
    }
}

/// Adds a screenshot to the card; false when the app has none.
fn add_screenshot(card: &mut AppCard) -> bool {
    if card.screenshot.is_none() && card.sources.iter().any(|s| s == "flatpak") {
        card.screenshot = flathub::app(&card.id).and_then(|app| flathub::screenshots(&app).into_iter().next());
    }
    if card.screenshot.is_none() {
        card.screenshot = apt::find(&card.id, &card.name).and_then(|app| app.screenshots.first().cloned());
    }
    card.screenshot.is_some()
}

/// Slides in the banner of every section and category page.
const BANNER_SLIDES: usize = 5;

/// The first apps with a screenshot go to the carousel, the rest below.
fn build_section(id: &str, cards: Vec<AppCard>, carousel: usize, grid: usize) -> Section {
    let mut featured = Vec::new();
    let mut apps = Vec::new();
    for (n, mut card) in cards.into_iter().enumerate() {
        if featured.len() < carousel && n < carousel + 6 && add_screenshot(&mut card) {
            featured.push(card);
        } else {
            apps.push(card);
        }
    }
    apps.truncate(grid);
    Section { id: id.to_string(), featured, apps }
}

/// Apps of one category from every source, like GNOME Software and
/// Discover do: Flathub's list plus Debian's AppStream metadata, merged
/// into one card per app and ordered by number of ODRS ratings (the same
/// measure for all sources). Ties keep Flathub's own order first.
fn category_cards(cat: &str, main: &str, flathub_count: u32, state: &InstalledState) -> Vec<AppCard> {
    let hits = flathub::collection(&format!("collection/category/{cat}"), flathub_count);
    // Debian apps already merged into a Flathub card are not listed twice.
    let merged: HashSet<String> = hits.iter().filter_map(|h| apt::find(&h.id, &h.name)).map(|a| a.id.clone()).collect();
    let mut cards: Vec<AppCard> = hits.iter().filter_map(|hit| card_from_hit(hit, state)).collect();
    cards.extend(
        apt::in_category(main)
            .into_iter()
            .filter(|app| !merged.contains(&app.id))
            .map(|app| card_from_apt(app, state)),
    );
    // Stable sort, so equal counts keep the order above.
    cards.sort_by(|a, b| b.reviews.cmp(&a.reviews));
    cards
}

/// The card of an app known by its Flathub ID.
fn card_for_id(id: &str, state: &InstalledState) -> Option<AppCard> {
    let app = flathub::app(id)?;
    card_from_hit(&flathub::hit_from_app(id, &app), state)
}

/// Featured: this week's picks in the banner and, below it, cards with
/// today's app and the picks of the week before, so the banner is not
/// followed straight by the next one.
fn featured_section(state: &InstalledState) -> Section {
    let featured: Vec<AppCard> = flathub::apps_of_the_week(0)
        .iter()
        .filter_map(|id| {
            let app = flathub::app(id)?;
            let mut card = card_from_hit(&flathub::hit_from_app(id, &app), state)?;
            card.screenshot = flathub::screenshots(&app).into_iter().next();
            Some(card)
        })
        .collect();

    let mut shown: HashSet<String> = featured.iter().map(|c| c.id.clone()).collect();
    let mut apps: Vec<AppCard> = Vec::new();
    let candidates = flathub::app_of_the_day().into_iter().chain(flathub::apps_of_the_week(7));
    for id in candidates {
        if apps.len() >= 6 {
            break;
        }
        if !shown.insert(id.clone()) {
            continue;
        }
        if let Some(card) = card_for_id(&id, state) {
            apps.push(card);
        }
    }
    Section { id: "featured".into(), featured, apps }
}

fn explore() -> Vec<Section> {
    let state = installed_state();
    // Featured, popular and trending are Flathub's curated and measured
    // lists; every category mixes all sources (see category_cards).
    let mut specs: Vec<(&str, Option<&str>)> = vec![("featured", None), ("popular", None), ("trending", None)];
    for (id, main) in CATEGORIES {
        specs.push((id, Some(main)));
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = specs
            .iter()
            .map(|&(id, main)| {
                let state = &state;
                scope.spawn(move || match (id, main) {
                    ("featured", _) => featured_section(state),
                    (_, Some(main)) => build_section(id, category_cards(id, main, 30, state), BANNER_SLIDES, 6),
                    _ => {
                        let path = format!("collection/{id}");
                        let cards = flathub::collection(&path, 15).iter().filter_map(|hit| card_from_hit(hit, state)).collect();
                        build_section(id, cards, BANNER_SLIDES, 6)
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|h| h.join().ok())
            .filter(|s| !s.featured.is_empty() || !s.apps.is_empty())
            .collect()
    })
}

fn category(cat: &str) -> Section {
    let state = installed_state();
    let Some((_, main)) = CATEGORIES.iter().find(|(id, _)| *id == cat) else {
        return Section { id: cat.to_string(), featured: Vec::new(), apps: Vec::new() };
    };
    let mut cards = category_cards(cat, main, 48, &state);
    cards.truncate(150);
    build_section(cat, cards, BANNER_SLIDES, usize::MAX)
}

fn search(query: &str) -> Vec<AppCard> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    let state = installed_state();
    let (flathub_hits, apt_hits, snap_hits) = std::thread::scope(|scope| {
        let f = scope.spawn(|| flathub::search(query));
        let a = scope.spawn(|| apt::search(query));
        let s = scope.spawn(|| snap::search(query));
        (f.join().unwrap_or_default(), a.join().unwrap_or_default(), s.join().unwrap_or_default())
    });

    let mut cards: Vec<AppCard> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for hit in flathub_hits.iter().take(30) {
        if let Some(card) = card_from_hit(hit, &state) {
            seen.insert(hit.id.clone());
            if let Some(app) = apt::find(&hit.id, &hit.name) {
                seen.insert(app.id.clone());
            }
            cards.push(card);
        }
    }
    for app in apt_hits {
        if seen.insert(app.id.clone()) {
            cards.push(card_from_apt(app, &state));
        }
    }
    // Snaps have no AppStream ID, so they are matched by name.
    for found in &snap_hits {
        let key = normalize_name(&found.title);
        match cards.iter_mut().find(|c| normalize_name(&c.name) == key) {
            Some(card) => {
                if !card.sources.iter().any(|s| s == "snap") {
                    card.sources.push("snap".into());
                    card.refs.push(snap_ref(&found.name, &state));
                }
                card.installed |= state.snap.contains(&found.name);
            }
            None => cards.push(card_from_snap(found, &state)),
        }
    }

    let q = query.to_lowercase();
    cards.sort_by_key(|card| {
        let name = card.name.to_lowercase();
        if name == q {
            0
        } else if name.starts_with(&q) {
            1
        } else if name.contains(&q) {
            2
        } else {
            3
        }
    });
    cards
}

/// Finds the snap of an app that is not on the Snap Store under its ID.
/// Snap is only offered as a source when snapd is installed.
fn find_snap(detail: &AppDetail, apt_package: Option<&str>) -> Option<(String, Value)> {
    if !snap::available() {
        return None;
    }
    let mut candidates: Vec<String> = Vec::new();
    if let Some(last) = detail.id.rsplit('.').next() {
        candidates.push(last.to_lowercase());
    }
    if let Some(package) = apt_package {
        candidates.push(package.to_string());
    }
    candidates.push(detail.name.to_lowercase().replace(' ', "-"));
    candidates.dedup();
    let wanted = normalize_name(&detail.name);
    candidates.into_iter().find_map(|name| {
        let info = snap::info(&name)?;
        let title = normalize_name(&snap::title(&info));
        (title == wanted || title == normalize_name(&name)).then_some((name, info))
    })
}

fn snap_detail(name: &str, state: &InstalledState) -> Option<AppDetail> {
    let info = snap::info(name)?;
    Some(AppDetail {
        id: format!("snap:{name}"),
        name: snap::title(&info),
        summary: snap::field(&info, "summary"),
        description: crate::text::text_paragraphs(&snap::field(&info, "description")),
        developer: snap::publisher(&info),
        license: snap::field(&info, "license"),
        homepage: snap::field(&info, "website"),
        icon: snap::icon(&info),
        screenshots: snap::screenshots(&info),
        sources: vec![SourceOption {
            kind: "snap".into(),
            id: name.to_string(),
            version: snap::version(&info),
            installed: state.snap.contains(name),
            download_size: snap::download_size(&info).map(system::format_size).unwrap_or_default(),
            ..Default::default()
        }],
        ..Default::default()
    })
}

fn detail(id: &str) -> Option<AppDetail> {
    let state = installed_state();
    if let Some(name) = id.strip_prefix("snap:") {
        return snap_detail(name, &state);
    }
    let flat = flathub::app(id);
    let flathub_name = flat.as_ref().map(|app| flathub::str_field(app, "name")).unwrap_or_default();
    let apt_app = apt::find(id, &flathub_name);
    if flat.is_none() && apt_app.is_none() {
        return None;
    }

    let mut detail = AppDetail { id: id.to_string(), ..Default::default() };
    if let Some(app) = &flat {
        detail.name = flathub::str_field(app, "name");
        detail.summary = flathub::str_field(app, "summary");
        detail.description = flathub::description(app);
        detail.developer = flathub::str_field(app, "developer_name");
        detail.license = flathub::str_field(app, "project_license");
        detail.homepage = flathub::homepage(app);
        detail.icon = app.get("icon").and_then(|i| i.as_str()).map(String::from);
        detail.screenshots = flathub::screenshots(app);
        detail.categories = flathub::categories(app);
        detail.links = flathub::links(app);
        detail.release = flathub::release(app);
    }
    if let Some(app) = apt_app {
        // Debian's metadata is translated, so its texts win outside English.
        let translated = system::language() != "en";
        let fill = |field: &mut String, value: &str, prefer: bool| {
            if !value.is_empty() && (field.is_empty() || prefer) {
                *field = value.to_string();
            }
        };
        fill(&mut detail.name, &app.name, translated);
        fill(&mut detail.summary, &app.summary, translated);
        fill(&mut detail.developer, &app.developer, false);
        fill(&mut detail.license, &app.license, false);
        fill(&mut detail.homepage, &app.homepage, false);
        if !app.description.is_empty() && (detail.description.is_empty() || translated) {
            detail.description = app.description.clone();
        }
        if detail.icon.is_none() {
            detail.icon = app.icon.clone();
        }
        if detail.screenshots.is_empty() {
            detail.screenshots = app.screenshots.clone();
        }
        // Whatever Flathub did not tell comes from Debian's metadata.
        if detail.categories.is_empty() {
            detail.categories = app.categories.clone();
        }
        for link in &app.links {
            if !detail.links.iter().any(|l| l.kind == link.kind) {
                detail.links.push(link.clone());
            }
        }
        if detail.release.is_none() {
            detail.release = app.release.clone();
        }
    }

    if let Some(app) = apt_app {
        let installed = state.apt.get(&app.package);
        let (download, installed_size) = apt::package_sizes(&app.package).unwrap_or_default();
        detail.sources.push(SourceOption {
            kind: "apt".into(),
            id: app.package.clone(),
            version: installed.map(|(v, _)| v.clone()).unwrap_or_else(|| app.version.clone()),
            installed: installed.is_some(),
            download_size: if download > 0 { system::format_size(download) } else { String::new() },
            installed_size: if installed_size > 0 { system::format_size(installed_size) } else { String::new() },
        });
    }
    // Flathub's metadata (texts, icon, screenshots) is used either way, but
    // Flatpak is only offered as a source when flatpak is installed.
    if let (Some(app), true) = (&flat, flatpak::available()) {
        let (download, installed_size) = flathub::sizes(id).unwrap_or_default();
        detail.sources.push(SourceOption {
            kind: "flatpak".into(),
            id: id.to_string(),
            version: flathub::version(app),
            installed: state.flatpak.contains(id),
            download_size: if download > 0 { system::format_size(download) } else { String::new() },
            installed_size: if installed_size > 0 { system::format_size(installed_size) } else { String::new() },
        });
    }
    if let Some((name, info)) = find_snap(&detail, apt_app.map(|a| a.package.as_str())) {
        detail.sources.push(SourceOption {
            kind: "snap".into(),
            version: snap::version(&info),
            installed: state.snap.contains(&name),
            id: name,
            download_size: snap::download_size(&info).map(system::format_size).unwrap_or_default(),
            ..Default::default()
        });
    }

    detail.rating_stars = best_full_rating(id, apt_app).map(|found| found.stars.to_vec()).unwrap_or_default();
    let (rating, reviews) = best_rating(id, apt_app);
    detail.rating = rating;
    detail.reviews = reviews;
    Some(detail)
}

fn installed_apps() -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    for (package, (version, size_kib)) in apt::installed_packages().iter() {
        if let Some(app) = apt::app_for_package(package) {
            apps.push(InstalledApp {
                id: app.id.clone(),
                name: app.name.clone(),
                summary: app.summary.clone(),
                icon: app.icon.clone(),
                source: "apt".into(),
                package: package.clone(),
                version: version.clone(),
                size: system::format_size(size_kib * 1024),
            });
        }
    }
    for app in flatpak::installed() {
        apps.push(InstalledApp {
            icon: flatpak::icon(&app.id, &app.scope),
            package: app.id.clone(),
            id: app.id,
            name: app.name,
            summary: app.summary,
            source: "flatpak".into(),
            version: app.version,
            size: app.size,
        });
    }
    for app in snap::installed() {
        apps.push(InstalledApp {
            id: format!("snap:{}", app.name),
            icon: snap::local_icon(&app.name),
            package: app.name.clone(),
            name: app.name,
            summary: String::new(),
            source: "snap".into(),
            version: app.version,
            size: String::new(),
        });
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

fn updates() -> Vec<UpdateItem> {
    // The three sources are asked at the same time, and the two that use
    // the network have a time limit, so one slow answer cannot hold the
    // whole screen.
    let (apt_updates, flatpak_updates, snap_updates, firmware_updates) = std::thread::scope(|scope| {
        let a = scope.spawn(apt::upgradable);
        let f = scope.spawn(flatpak::updates);
        let s = scope.spawn(snap::updates);
        let w = scope.spawn(fwupd::updates);
        (a.join().unwrap_or_default(), f.join().unwrap_or_default(), s.join().unwrap_or_default(), w.join().unwrap_or_default())
    });
    let mut items = Vec::new();
    for (package, from, to) in apt_updates {
        let app = apt::app_for_package(&package);
        items.push(UpdateItem {
            id: package.clone(),
            name: app.map(|a| a.name.clone()).unwrap_or(package),
            icon: app.and_then(|a| a.icon.clone()),
            source: "apt".into(),
            from,
            to,
        });
    }
    let installed_flatpaks: HashMap<String, flatpak::Installed> = flatpak::installed().into_iter().map(|a| (a.id.clone(), a)).collect();
    for (id, name, to) in flatpak_updates {
        let local = installed_flatpaks.get(&id);
        items.push(UpdateItem {
            icon: local.and_then(|a| flatpak::icon(&a.id, &a.scope)),
            from: local.map(|a| a.version.clone()).unwrap_or_default(),
            name: if name.is_empty() { id.clone() } else { name },
            id,
            source: "flatpak".into(),
            to,
        });
    }
    let installed_snaps: HashMap<String, String> = snap::installed().into_iter().map(|s| (s.name, s.version)).collect();
    for (name, to) in snap_updates {
        items.push(UpdateItem {
            icon: snap::local_icon(&name),
            from: installed_snaps.get(&name).cloned().unwrap_or_default(),
            id: name.clone(),
            name,
            source: "snap".into(),
            to,
        });
    }
    for update in firmware_updates {
        items.push(UpdateItem { id: update.device_id, name: update.name, icon: None, source: "firmware".into(), from: update.from, to: update.to });
    }
    // The tray shows the stored count, so the page keeps it up to date.
    checker::store(items.len());
    items
}

#[tauri::command]
pub async fn get_explore() -> Vec<Section> {
    tauri::async_runtime::spawn_blocking(explore).await.unwrap_or_default()
}

#[tauri::command]
pub async fn get_category(category: String) -> Option<Section> {
    tauri::async_runtime::spawn_blocking(move || self::category(&category)).await.ok()
}

#[tauri::command]
pub async fn search_apps(query: String) -> Vec<AppCard> {
    tauri::async_runtime::spawn_blocking(move || search(&query)).await.unwrap_or_default()
}

#[tauri::command]
pub async fn get_app_detail(id: String) -> Option<AppDetail> {
    tauri::async_runtime::spawn_blocking(move || detail(&id)).await.ok().flatten()
}

#[tauri::command]
pub async fn get_installed() -> Vec<InstalledApp> {
    tauri::async_runtime::spawn_blocking(installed_apps).await.unwrap_or_default()
}

#[tauri::command]
pub async fn get_updates() -> Vec<UpdateItem> {
    tauri::async_runtime::spawn_blocking(updates).await.unwrap_or_default()
}

/// State of apt's package lists, to tell the user when they are stale.
#[tauri::command]
pub async fn get_apt_status() -> Option<AptStatus> {
    tauri::async_runtime::spawn_blocking(|| {
        let (lists_age_secs, app_count) = apt::lists_status();
        AptStatus { lists_age_secs, app_count }
    })
    .await
    .ok()
}

/// Reads Debian's metadata again after the package lists were refreshed.
#[tauri::command]
pub async fn reload_apt() {
    let _ = tauri::async_runtime::spawn_blocking(|| {
        apt::reload();
        apt::invalidate_installed();
    })
    .await;
}

#[tauri::command]
pub async fn get_sources_status() -> Option<SourcesStatus> {
    tauri::async_runtime::spawn_blocking(|| SourcesStatus {
        flatpak: flatpak::available(),
        flathub: flatpak::flathub_scope(),
        snap: snap::available(),
        repo_selector: system::has_command("soplos-repo-selector"),
        appimage_manager: system::has_command("soplos-appimage-manager"),
        fwupd: fwupd::available(),
        fwupd_testing: fwupd::testing_enabled(),
    })
    .await
    .ok()
}

/// Parses Debian's metadata and downloads the ratings in the background,
/// so the first screen does not have to wait for them.
pub fn prewarm() {
    std::thread::spawn(|| {
        let _ = apt::index();
        let _ = odrs::lookup("");
    });
}
