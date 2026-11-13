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
  // Every package that removing it has to remove, when it is a suite of many (empty otherwise).
  packages: string[];
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

/** A graphics card of this machine and the driver that fits it. */
export interface GpuInfo {
  /** "NVIDIA", "AMD", "Intel" or "Virtual". */
  vendor: string;
  model: string;
  /** "Dedicated", "Integrated", "Dedicated/Integrated" or "Virtual". */
  gpuType: string;
  /** For NVIDIA a branch ("nvidia-driver-620", "nouveau"...), for AMD and Intel the packages, space separated. */
  recommendedDriver: string | null;
  driverStatus: "installed" | "different_version" | "partial" | "missing";
  installedDriverVersion: string | null;
}

/** Whether an NVIDIA driver branch is the one installed. */
export interface NvidiaBranch {
  id: string;
  installed: boolean;
}

/** A printer plugged in by USB; `brand` is empty when it is not one of the brands of the page. */
export interface PrinterInfo {
  brand: string;
  vendor: string;
  model: string;
}

/** The Debian packages of one brand and whether all of them are installed. */
export interface PackageSet {
  id: string;
  packages: string[];
  installed: boolean;
}

/** A package that comes in or goes out together with the updates (a renamed library, say). */
export interface PlanChange {
  name: string;
  version: string;
}

/** The updates of the last check, what comes in and goes out with them, and when it was made (epoch seconds). */
export interface UpdatesView {
  items: UpdateItem[];
  added: PlanChange[];
  removed: PlanChange[];
  checkedAt: number;
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
  /** The result of the last check, instantly (it only looks again when there is none). */
  updates: () => track(invoke<UpdatesView>("get_updates")),
  /** Looks in every source again, without "apt update", and keeps the result. */
  rescanUpdates: () => track(invoke<UpdatesView>("rescan_updates")),
  gpus: () => track(invoke<GpuInfo[]>("get_gpus")),
  nvidiaBranches: () => track(invoke<NvidiaBranch[]>("get_nvidia_branches")),
  driverPackages: () => track(invoke<PackageSet[]>("get_driver_packages")),
  printers: () => track(invoke<PrinterInfo[]>("get_printers")),
  printerPackages: () => track(invoke<PackageSet[]>("get_printer_packages")),
  packageSets: (sets: { id: string; packages: string[] }[]) => track(invoke<PackageSet[]>("get_package_sets", { sets })),
  sources: () => invoke<SourcesStatus | null>("get_sources_status"),
  aptStatus: () => invoke<{ listsAgeSecs: number | null; appCount: number } | null>("get_apt_status"),
  environment: () => invoke<{ desktop: string; session: string }>("get_environment_info"),
  launchApp: (name: "soplos-repo-selector" | "soplos-appimage-manager") => invoke<void>("launch_app", { name }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  takeStartupFile: () => invoke<string | null>("take_startup_file"),
  takeStartupUpdates: () => invoke<boolean>("take_startup_updates"),
  inspectPackage: (path: string) => track(invoke<LocalPackage | null>("inspect_package", { path })),
};
