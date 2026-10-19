import { invoke } from "@tauri-apps/api/core";
import { track } from "./busy";

export type SourceKind = "apt" | "flatpak" | "snap" | "firmware" | "appimage";

export interface AppCard {
  id: string;
  name: string;
  summary: string;
  icon: string | null;
  screenshot: string | null;
  sources: SourceKind[];
  refs: SourceOption[];
  rating: number | null;
  reviews: number;
  installed: boolean;
  categories: string[];
}

export interface Section {
  id: string;
  featured: AppCard[];
  apps: AppCard[];
}

export interface SourceOption {
  kind: SourceKind;
  id: string;
  version: string;
  installed: boolean;
  downloadSize: string;
  installedSize: string;
}

export interface AppLink {
  kind: "homepage" | "help" | "bugtracker" | "donation" | "source";
  url: string;
}

export interface ReleaseInfo {
  version: string;
  date: string;
  notes: string[];
}

export interface AppDetail {
  id: string;
  name: string;
  summary: string;
  description: string[];
  developer: string;
  license: string;
  homepage: string;
  icon: string | null;
  screenshots: string[];
  sources: SourceOption[];
  categories: string[];
  links: AppLink[];
  release: ReleaseInfo | null;
  rating: number | null;
  reviews: number;
  ratingStars: number[];
}

export interface InstalledApp {
  id: string;
  name: string;
  summary: string;
  icon: string | null;
  source: SourceKind;
  package: string;
  version: string;
  size: string;
}

export interface UpdateItem {
  id: string;
  name: string;
  icon: string | null;
  source: SourceKind;
  from: string;
  to: string;
}

export interface SourcesStatus {
  flatpak: boolean;
  flathub: string;
  snap: boolean;
  repoSelector: boolean;
  appimageManager: boolean;
  fwupd: boolean;
  fwupdTesting: boolean;
}

export interface LocalPackage {
  path: string;
  fileName: string;
  kind: "deb" | "flatpak" | "flatpakref" | "snap" | "appimage";
  name: string;
  version: string;
  summary: string;
  size: string;
}

// Every call shows the progress strip at the bottom while it runs.
export const api = {
  explore: () => track(invoke<Section[]>("get_explore")),
  category: (category: string) => track(invoke<Section | null>("get_category", { category })),
  search: (query: string) => track(invoke<AppCard[]>("search_apps", { query })),
  detail: (id: string) => track(invoke<AppDetail | null>("get_app_detail", { id })),
  installed: () => track(invoke<InstalledApp[]>("get_installed")),
  updates: () => track(invoke<UpdateItem[]>("get_updates")),
  sources: () => invoke<SourcesStatus | null>("get_sources_status"),
  aptStatus: () => invoke<{ listsAgeSecs: number | null; appCount: number } | null>("get_apt_status"),
  environment: () => invoke<{ desktop: string; session: string }>("get_environment_info"),
  launchApp: (name: "soplos-repo-selector" | "soplos-appimage-manager") => invoke<void>("launch_app", { name }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  takeStartupFile: () => invoke<string | null>("take_startup_file"),
  takeStartupUpdates: () => invoke<boolean>("take_startup_updates"),
  inspectPackage: (path: string) => track(invoke<LocalPackage | null>("inspect_package", { path })),
};
