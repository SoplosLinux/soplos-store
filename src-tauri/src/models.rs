use serde::Serialize;

/// One app as shown in tiles, carousels and search results. The same app
/// found in several sources (APT, Flatpak, Snap) is merged into one card.
#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppCard {
    /// AppStream ID without the ".desktop" suffix.
    pub id: String,
    pub name: String,
    pub summary: String,
    pub icon: Option<String>,
    pub screenshot: Option<String>,
    /// "apt", "flatpak" and/or "snap".
    pub sources: Vec<String>,
    /// What to install or remove from each source.
    pub refs: Vec<SourceOption>,
    pub rating: Option<f32>,
    pub reviews: u32,
    pub installed: bool,
    /// AppStream categories, for the subcategory filter of a category page.
    pub categories: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub id: String,
    pub featured: Vec<AppCard>,
    pub apps: Vec<AppCard>,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceOption {
    pub kind: String,
    /// Package name, Flatpak app ID or snap name.
    pub id: String,
    pub version: String,
    pub installed: bool,
    /// Human readable sizes; empty when the source does not tell.
    pub download_size: String,
    pub installed_size: String,
}

/// A project link: kind is homepage, help, bugtracker, donation or source.
#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub kind: String,
    pub url: String,
}

/// Latest release of an app, with its notes as plain paragraphs.
#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    pub version: String,
    pub date: String,
    pub notes: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AptStatus {
    /// Seconds since apt last refreshed its lists; None when it never did.
    pub lists_age_secs: Option<u64>,
    /// Desktop apps found in the Debian metadata.
    pub app_count: usize,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppDetail {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub description: Vec<String>,
    pub developer: String,
    pub license: String,
    pub homepage: String,
    pub icon: Option<String>,
    pub screenshots: Vec<String>,
    pub sources: Vec<SourceOption>,
    pub categories: Vec<String>,
    pub links: Vec<Link>,
    pub release: Option<ReleaseInfo>,
    pub rating: Option<f32>,
    pub reviews: u32,
    /// Reviews with 1 to 5 stars; empty when the app has no ratings.
    pub rating_stars: Vec<u32>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub icon: Option<String>,
    pub source: String,
    /// Package name, Flatpak app ID or snap name, for removal.
    pub package: String,
    pub version: String,
    pub size: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateItem {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub source: String,
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SourcesStatus {
    pub flatpak: bool,
    /// "user", "system", "both" or "" when no Flathub remote exists.
    pub flathub: String,
    pub snap: bool,
    pub repo_selector: bool,
    pub appimage_manager: bool,
    pub fwupd: bool,
    /// The LVFS testing channel is enabled.
    pub fwupd_testing: bool,
}
