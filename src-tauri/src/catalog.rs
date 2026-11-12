// Joins APT, Flatpak and Snap into one catalog: one card per app, listing
// every source it can be installed from. All commands run on the
// blocking thread pool so the window never freezes while loading.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::apt::{self, AptApp};
use crate::flathub::{self, Hit};
use crate::models::{AppCard, AppDetail, AptStatus, InstalledApp, PlanChange, Section, SourceOption, SourcesStatus, UpdateItem, UpdatesView};
use crate::{appimage, appimage_update, checker, desktop, flatpak, fwupd, odrs, settings, snap, system};

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
    // APT first: it is the system's own source, and the app page starts on
    // it; the user picks another source there.
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
        screenshot: found.screenshot.clone(),
        sources: vec!["snap".into()],
        installed: reference.installed,
        refs: vec![reference],
        rating: None,
        reviews: 0,
        categories: Vec::new(),
    }
}

/// Snap Store categories that match a category of the Store.
fn snap_categories(cat: &str) -> &'static [&'static str] {
    match cat {
        "audiovideo" => &["music-and-audio", "entertainment"],
        "development" => &["development"],
        "education" => &["education"],
        "game" => &["games"],
        "graphics" => &["photo-and-video", "art-and-design"],
        "network" => &["social"],
        "office" => &["productivity", "finance"],
        "science" => &["science"],
        "system" => &["security", "devices-and-iot"],
        "utility" => &["utilities"],
        _ => &[],
    }
}

/// The snaps of the Snap Store categories that match a category of the Store;
/// empty without snapd.
fn snaps_of(cat: &str) -> Vec<snap::StoreSnap> {
    let mut seen = HashSet::new();
    snap_categories(cat).iter().flat_map(|name| snap::category(name)).filter(|found| seen.insert(found.name.clone())).collect()
}

/// Puts snaps among the cards: one with the name of an app that is already
/// there becomes one more source of that card, the others get their own.
fn add_snaps(cards: &mut Vec<AppCard>, snaps: Vec<snap::StoreSnap>, state: &InstalledState) {
    for found in &snaps {
        let key = normalize_name(&found.title);
        match cards.iter_mut().find(|card| normalize_name(&card.name) == key) {
            Some(card) => {
                if !card.sources.iter().any(|s| s == "snap") {
                    card.sources.push("snap".into());
                    card.refs.push(snap_ref(&found.name, state));
                }
                card.installed |= state.snap.contains(&found.name);
            }
            None => cards.push(card_from_snap(found, state)),
        }
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

/// Apps of each source (the first ones of each) looked at for the banner of a section.
const BANNER_CANDIDATES: usize = 40;

/// Of the best apps of each source, how many take turns from week to week.
const BANNER_ROTATION: usize = 10;

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// The number of the week, so the banners change from one week to the next.
fn week() -> usize {
    (now_secs() / (7 * 86_400)) as usize
}

/// Where a card comes from, for sharing the banner out: Debian first (also
/// when Flatpak has the app too), then Flatpak, then Snap.
fn source_group(card: &AppCard) -> usize {
    if card.sources.iter().any(|s| s == "apt") {
        0
    } else if card.sources.iter().any(|s| s == "flatpak") {
        1
    } else {
        2
    }
}

/// One of each in turn, so no source fills a section on its own.
fn interleave(first: Vec<AppCard>, second: Vec<AppCard>) -> Vec<AppCard> {
    let mut first = first.into_iter();
    let mut second = second.into_iter();
    let mut mixed = Vec::new();
    loop {
        let (a, b) = (first.next(), second.next());
        if a.is_none() && b.is_none() {
            break;
        }
        mixed.extend(a);
        mixed.extend(b);
    }
    mixed
}

/// The banner takes the apps with a screenshot of every source in turn (APT,
/// Flatpak, Snap), each in the order of the section, and the first ones of
/// each source take turns week by week. What is left goes below, in the
/// order it came.
fn build_section(id: &str, cards: Vec<AppCard>, carousel: usize, grid: usize) -> Section {
    let mut slots: Vec<Option<AppCard>> = cards.into_iter().map(Some).collect();
    let mut groups: [Vec<usize>; 3] = Default::default();
    for (i, slot) in slots.iter().enumerate() {
        if let Some(card) = slot {
            let group = &mut groups[source_group(card)];
            if group.len() < BANNER_CANDIDATES {
                group.push(i);
            }
        }
    }
    for group in &mut groups {
        let top = group.len().min(BANNER_ROTATION);
        if top > 1 {
            group[..top].rotate_left(week() % top);
        }
    }

    let mut featured = Vec::new();
    let mut next = [0usize; 3];
    'fill: while featured.len() < carousel {
        let mut taken = false;
        for g in 0..groups.len() {
            if featured.len() >= carousel {
                break 'fill;
            }
            while next[g] < groups[g].len() {
                let i = groups[g][next[g]];
                next[g] += 1;
                let Some(card) = slots[i].as_mut() else {
                    continue;
                };
                if add_screenshot(card) {
                    if let Some(card) = slots[i].take() {
                        featured.push(card);
                    }
                    taken = true;
                    break;
                }
            }
        }
        if !taken {
            break;
        }
    }

    let mut apps: Vec<AppCard> = slots.into_iter().flatten().collect();
    apps.truncate(grid);
    Section { id: id.to_string(), featured, apps }
}

/// Apps of one category from every source, like GNOME Software and
/// Discover do: Flathub's list plus Debian's AppStream metadata, merged
/// into one card per app and ordered by number of ODRS ratings (the same
/// measure for all sources). Ties keep Flathub's own order first.
fn category_cards(cat: &str, main: &str, flathub_count: u32, state: &InstalledState) -> Vec<AppCard> {
    // Flathub is only asked when Flatpak is installed: without it the
    // category is made of what the other sources have.
    let hits = if flatpak::available() { flathub::collection(&format!("collection/category/{cat}"), flathub_count) } else { Vec::new() };
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

/// Apps of Debian that the ODRS ratings put first in a section. ODRS is keyed
/// by AppStream ID and takes ratings from users of every distribution, so
/// this is a global measure; only the apps of this machine's catalog are
/// listed.
///   featured: the best average, with a minimum of reviews;
///   trending: the ones with a version released in the last 30 days;
///   popular (and the rest): the most reviews.
fn ranked_apt_apps(section: &str) -> Vec<&'static AptApp> {
    const BEST_MIN_REVIEWS: u32 = 20;
    const RECENT_DAYS: u64 = 30;
    const RANKED: usize = 60;
    let since = system::date_from_secs(now_secs().saturating_sub(RECENT_DAYS * 86_400));
    let mut ranked: Vec<(&'static AptApp, odrs::Rating)> = apt::index()
        .values()
        .filter_map(|app| best_full_rating(&app.id, Some(app)).map(|rating| (app, rating)))
        .filter(|(app, rating)| match section {
            "featured" => rating.count >= BEST_MIN_REVIEWS,
            "trending" => app.release.as_ref().is_some_and(|release| release.date.as_str() >= since.as_str()),
            _ => true,
        })
        .collect();
    ranked.sort_by(|a, b| {
        let by = if section == "featured" {
            b.1.average.partial_cmp(&a.1.average).unwrap_or(Ordering::Equal).then(b.1.count.cmp(&a.1.count))
        } else {
            b.1.count.cmp(&a.1.count)
        };
        by.then_with(|| a.0.name.cmp(&b.0.name))
    });
    ranked.into_iter().take(RANKED).map(|(app, _)| app).collect()
}

/// Debian apps of a ranking, left out the ones already in `taken`, as cards.
fn apt_cards(section: &str, taken: &HashSet<String>, state: &InstalledState) -> Vec<AppCard> {
    ranked_apt_apps(section).into_iter().filter(|app| !taken.contains(&app.id)).map(|app| card_from_apt(app, state)).collect()
}

/// The Debian app of each card, so it is not listed a second time.
fn debian_ids(cards: &[AppCard]) -> HashSet<String> {
    cards.iter().filter_map(|card| apt::find(&card.id, &card.name)).map(|app| app.id.clone()).collect()
}

/// Featured: this week's picks of Flathub (only when Flatpak is installed),
/// the best rated apps of Debian, in turn, and the featured snaps of the Snap
/// Store (only when snapd is installed). The banner takes all of them.
fn featured_section(state: &InstalledState) -> Section {
    let mut flat: Vec<AppCard> = Vec::new();
    if flatpak::available() {
        let candidates = flathub::apps_of_the_week(0).into_iter().chain(flathub::app_of_the_day()).chain(flathub::apps_of_the_week(7));
        let mut shown: HashSet<String> = HashSet::new();
        for id in candidates {
            if flat.len() >= 12 {
                break;
            }
            if !shown.insert(id.clone()) {
                continue;
            }
            if let Some(card) = card_for_id(&id, state) {
                flat.push(card);
            }
        }
    }
    let taken = debian_ids(&flat);
    let debian = apt_cards("featured", &taken, state);
    let mut cards = interleave(flat, debian);
    add_snaps(&mut cards, snap::category("featured"), state);
    build_section("featured", cards, BANNER_SLIDES, 6)
}

/// Popular and trending: the list of Flathub (only when Flatpak is installed)
/// and the ranking of Debian apps, in turn.
fn list_cards(section: &str, state: &InstalledState) -> Vec<AppCard> {
    let hits = if flatpak::available() { flathub::collection(&format!("collection/{section}"), 15) } else { Vec::new() };
    let flat: Vec<AppCard> = hits.iter().filter_map(|hit| card_from_hit(hit, state)).collect();
    let taken = debian_ids(&flat);
    let debian = apt_cards(section, &taken, state);
    interleave(flat, debian)
}

fn explore() -> Vec<Section> {
    let state = installed_state();
    // Featured, popular and trending mix the lists of Flathub (when Flatpak
    // is installed) with rankings of Debian apps; every category mixes all
    // sources (see category_cards). The banner of each shares its slides out
    // between the sources (see build_section).
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
                    (_, Some(main)) => {
                        let mut cards = category_cards(id, main, 30, state);
                        add_snaps(&mut cards, snaps_of(id), state);
                        build_section(id, cards, BANNER_SLIDES, 6)
                    }
                    _ => build_section(id, list_cards(id, state), BANNER_SLIDES, 6),
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
    add_snaps(&mut cards, snaps_of(cat), &state);
    build_section(cat, cards, BANNER_SLIDES, usize::MAX)
}

fn search(query: &str) -> Vec<AppCard> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    let state = installed_state();
    let (flathub_hits, apt_hits, snap_hits) = std::thread::scope(|scope| {
        let f = scope.spawn(|| if flatpak::available() { flathub::search(query) } else { Vec::new() });
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
    // Without Flatpak the page is made of what the other sources say.
    let flat = if flatpak::available() { flathub::app(id) } else { None };
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
    let installed = apt::installed_packages();
    let desktop_fallback = desktop::desktop_apps();
    let mut seen_packages: HashSet<String> = HashSet::new();

    // 1. Suite unification: LibreOffice.
    // If the metapackage or libreoffice-common is installed, unify the entire
    // suite into a single "LibreOffice" entry instead of 6 loose sub-apps.
    let has_libreoffice = installed.contains_key("libreoffice") || installed.contains_key("libreoffice-common");
    if has_libreoffice {
        let lo_version = installed
            .get("libreoffice")
            .or_else(|| installed.get("libreoffice-common"))
            .or_else(|| installed.get("libreoffice-writer"))
            .map(|(v, _)| v.clone())
            .unwrap_or_default();
        let lo_size: u64 = installed
            .iter()
            .filter(|(pkg, _)| pkg.starts_with("libreoffice-") || *pkg == "libreoffice")
            .map(|(_, (_, size_kib))| size_kib * 1024)
            .sum();

        // Removing the entry removes every package of the suite that is installed, and not only
        // the metapackage that names it: the metapackage can already be gone while the rest stays.
        let mut suite: Vec<String> = installed.keys().filter(|pkg| pkg.starts_with("libreoffice-") || *pkg == "libreoffice").cloned().collect();
        suite.sort();

        let icon = desktop_fallback
            .get("libreoffice-common")
            .and_then(|e| e.icon.clone())
            .or_else(|| desktop::resolve_icon("libreoffice-startcenter"))
            .or_else(|| desktop::resolve_icon("libreoffice"));

        apps.push(InstalledApp {
            id: "libreoffice".into(),
            name: "LibreOffice".into(),
            summary: "Office productivity suite".into(),
            icon,
            source: "apt".into(),
            package: "libreoffice".into(),
            packages: suite,
            version: lo_version,
            size: system::format_size(lo_size),
        });

        // Mark all libreoffice packages so sub-apps never appear separately.
        for (pkg, _) in installed.iter() {
            if pkg.starts_with("libreoffice-") || pkg == "libreoffice" {
                seen_packages.insert(pkg.clone());
            }
        }
    }

    // 2. First pass: packages with DEP-11 AppStream metadata (rich data).
    for (package, (version, size_kib)) in installed.iter() {
        if seen_packages.contains(package) {
            continue;
        }
        if let Some(app) = apt::app_for_package(package) {
            seen_packages.insert(package.clone());
            apps.push(InstalledApp {
                id: app.id.clone(),
                name: app.name.clone(),
                summary: app.summary.clone(),
                icon: app.icon.clone(),
                source: "apt".into(),
                package: package.clone(),
                packages: Vec::new(),
                version: version.clone(),
                size: system::format_size(size_kib * 1024),
            });
        }
    }

    // 3. Second pass: packages that own a .desktop file but have no DEP-11
    // entry. This catches apps like KMail, GIMP or custom apps on systems
    // whose AppStream metadata is incomplete or absent.
    for (package, entry) in &desktop_fallback {
        if seen_packages.contains(package) {
            continue;
        }
        if let Some((version, size_kib)) = installed.get(package) {
            seen_packages.insert(package.clone());
            apps.push(InstalledApp {
                id: package.clone(),
                name: entry.name.clone(),
                summary: entry.comment.clone(),
                icon: entry.icon.clone(),
                source: "apt".into(),
                package: package.clone(),
                packages: Vec::new(),
                version: version.clone(),
                size: system::format_size(size_kib * 1024),
            });
        }
    }

    // 4. Flatpak apps.
    for app in flatpak::installed() {
        apps.push(InstalledApp {
            icon: flatpak::icon(&app.id, &app.scope),
            package: app.id.clone(),
            packages: Vec::new(),
            id: app.id,
            name: app.name,
            summary: app.summary,
            source: "flatpak".into(),
            version: app.version,
            size: app.size,
        });
    }

    // 5. Snap apps.
    for app in snap::installed() {
        apps.push(InstalledApp {
            id: format!("snap:{}", app.name),
            icon: snap::local_icon(&app.name),
            package: app.name.clone(),
            packages: Vec::new(),
            name: app.name,
            summary: String::new(),
            source: "snap".into(),
            version: app.version,
            size: String::new(),
        });
    }

    // 6. AppImage apps from ~/AppImages.
    let appimages = if settings::appimage_enabled() { appimage::installed() } else { Vec::new() };
    for app in appimages {
        apps.push(InstalledApp {
            id: format!("appimage:{}", app.file),
            name: app.name,
            summary: app.summary,
            icon: app.icon,
            source: "appimage".into(),
            package: app.file,
            packages: Vec::new(),
            version: app.version,
            size: app.size,
        });
    }

    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

/// What one source found and how long it took to answer.
pub struct SourceScan {
    pub label: &'static str,
    pub items: Vec<UpdateItem>,
    pub took: std::time::Duration,
    /// Packages that come in and go out with the updates; only APT has them.
    pub added: Vec<PlanChange>,
    pub removed: Vec<PlanChange>,
}

fn timed(label: &'static str, find: impl FnOnce() -> Vec<UpdateItem>) -> SourceScan {
    let start = std::time::Instant::now();
    let items = find();
    SourceScan { label, items, took: start.elapsed(), added: Vec::new(), removed: Vec::new() }
}

/// The updates apt would make, and the packages that come in and go out with them.
fn apt_updates() -> (Vec<UpdateItem>, Vec<PlanChange>, Vec<PlanChange>) {
    let plan = apt::pending();
    let items = plan
        .updates
        .into_iter()
        .map(|(package, from, to)| UpdateItem { id: package.clone(), name: package, icon: None, source: "apt".into(), from, to })
        .collect();
    let change = |(name, version): (String, String)| PlanChange { name, version };
    (items, plan.added.into_iter().map(change).collect(), plan.removed.into_iter().map(change).collect())
}

fn flatpak_updates() -> Vec<UpdateItem> {
    let found = flatpak::updates();
    if found.is_empty() {
        return Vec::new();
    }
    let versions: HashMap<String, String> = flatpak::installed().into_iter().map(|a| (a.id, a.version)).collect();
    found
        .into_iter()
        .map(|(id, name, to)| UpdateItem {
            from: versions.get(&id).cloned().unwrap_or_default(),
            name: if name.is_empty() { id.clone() } else { name },
            id,
            icon: None,
            source: "flatpak".into(),
            to,
        })
        .collect()
}

fn snap_updates() -> Vec<UpdateItem> {
    let found = snap::updates();
    if found.is_empty() {
        return Vec::new();
    }
    let versions: HashMap<String, String> = snap::installed().into_iter().map(|s| (s.name, s.version)).collect();
    found
        .into_iter()
        .map(|(name, to)| UpdateItem { from: versions.get(&name).cloned().unwrap_or_default(), id: name.clone(), name, icon: None, source: "snap".into(), to })
        .collect()
}

fn appimage_updates() -> Vec<UpdateItem> {
    appimage_update::updates()
        .into_iter()
        .map(|u| UpdateItem { id: u.file, name: u.name, icon: None, source: "appimage".into(), from: u.from, to: u.to })
        .collect()
}

fn firmware_updates() -> Vec<UpdateItem> {
    fwupd::updates()
        .into_iter()
        .map(|u| UpdateItem { id: u.device_id, name: u.name, icon: None, source: "firmware".into(), from: u.from, to: u.to })
        .collect()
}

/// The one check: every source is asked once, all at the same time and each
/// with its own time limit, so a slow answer cannot hold the others up. The
/// items carry no icon and the app name is the package name: the tray
/// notifier does this without loading Debian's app metadata, and `decorate`
/// adds both when the list is shown.
pub fn scan() -> Vec<SourceScan> {
    std::thread::scope(|scope| {
        let apt = scope.spawn(|| {
            let start = std::time::Instant::now();
            let (items, added, removed) = apt_updates();
            Some(SourceScan { label: "APT", items, took: start.elapsed(), added, removed })
        });
        let flatpak = scope.spawn(|| flatpak::available().then(|| timed("Flatpak", flatpak_updates)));
        let snap = scope.spawn(|| snap::available().then(|| timed("Snap", snap_updates)));
        let appimage = scope.spawn(|| (settings::appimage_enabled() && appimage_update::any_checkable()).then(|| timed("AppImage", appimage_updates)));
        // With firmware turned off in Settings fwupd is not asked at all.
        let firmware = scope.spawn(|| (fwupd::available() && settings::firmware_enabled()).then(|| timed("Firmware", firmware_updates)));
        [apt, flatpak, snap, appimage, firmware].into_iter().filter_map(|handle| handle.join().ok().flatten()).collect()
    })
}

/// Adds what the check leaves out, the name and the icon of each app, from
/// local data only: Debian's app metadata, the icons Flatpak and Snap
/// export and the launcher of each AppImage. No command and no network.
fn decorate(mut items: Vec<UpdateItem>) -> Vec<UpdateItem> {
    let appimage_icons: HashMap<String, Option<String>> = if items.iter().any(|i| i.source == "appimage") {
        appimage::installed().into_iter().map(|a| (a.file, a.icon)).collect()
    } else {
        HashMap::new()
    };
    for item in &mut items {
        match item.source.as_str() {
            "apt" => {
                if let Some(app) = apt::app_for_package(&item.id) {
                    item.name = app.name.clone();
                    item.icon = app.icon.clone();
                }
            }
            "flatpak" => item.icon = flatpak::icon(&item.id, "user").or_else(|| flatpak::icon(&item.id, "system")),
            "snap" => item.icon = snap::local_icon(&item.id),
            "appimage" => item.icon = appimage_icons.get(&item.id).cloned().flatten(),
            _ => {}
        }
    }
    items
}

/// The result of the last check, read from the file the tray uses too. It
/// is checked again only when asked to, or when there is no result yet.
fn updates_view(force: bool) -> UpdatesView {
    let stored = if force { None } else { checker::read().filter(|s| s.items.is_some()) };
    let stored = stored.unwrap_or_else(checker::scan_and_store);
    UpdatesView { checked_at: stored.checked_at, items: decorate(stored.items.unwrap_or_default()), added: stored.added, removed: stored.removed }
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

/// The updates of the last check, instantly.
#[tauri::command]
pub async fn get_updates() -> UpdatesView {
    tauri::async_runtime::spawn_blocking(|| updates_view(false)).await.unwrap_or_default()
}

/// Looks in every source again (no `apt update`) and stores the result.
#[tauri::command]
pub async fn rescan_updates() -> UpdatesView {
    tauri::async_runtime::spawn_blocking(|| updates_view(true)).await.unwrap_or_default()
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
        flatpak: flatpak::present(),
        flathub: flatpak::flathub_scope(),
        snap: snap::present(),
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

#[cfg(test)]
mod tests {
    use super::*;

    // These set XDG_CACHE_HOME, so they run one at a time:
    // cargo test updates_view -- --test-threads=1
    fn cache_in(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("soplos-store-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("XDG_CACHE_HOME", &dir);
        dir
    }

    fn firmware(id: &str) -> UpdateItem {
        UpdateItem { id: id.into(), name: format!("Device {id}"), icon: None, source: "firmware".into(), from: "1.0".into(), to: "2.0".into() }
    }

    #[test]
    fn the_page_shows_the_stored_result_without_looking_again() {
        let dir = cache_in("stored");
        let saved = checker::save(vec![firmware("a"), firmware("b")]);
        let view = updates_view(false);
        // A new look would have stored what this machine really has.
        assert_eq!(view.items.len(), 2);
        assert_eq!(view.checked_at, saved.checked_at);
        assert_eq!(checker::read().unwrap().count, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_tray_count_and_the_page_list_are_the_same_result() {
        let dir = cache_in("same");
        checker::save(vec![firmware("a"), firmware("b"), firmware("c")]);
        let count = checker::read().unwrap().count;
        assert_eq!(count, updates_view(false).items.len());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_written_before_the_list_was_kept_has_no_result() {
        let dir = cache_in("old");
        std::fs::write(crate::cache::dir("").join("updates.json"), br#"{"count":3,"checkedAt":5}"#).unwrap();
        let stored = checker::read().unwrap();
        assert!(stored.items.is_none());
        assert_eq!(stored.count, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[ignore]
    fn with_no_result_the_page_looks_once_and_keeps_it() {
        let dir = cache_in("none");
        std::fs::write(crate::cache::dir("").join("updates.json"), br#"{"count":3,"checkedAt":5}"#).unwrap();
        let view = updates_view(false);
        let stored = checker::read().unwrap();
        assert!(stored.items.is_some());
        assert_eq!(stored.count, view.items.len());
        assert!(stored.checked_at > 5);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
