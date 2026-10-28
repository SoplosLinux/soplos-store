import { writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

// User preferences. Stored locally for now; the update timer and the
// install backend read them once those parts exist.
export interface Settings {
  flatpakScope: "user" | "system" | "ask";
  autoCheck: boolean;
  frequency: "6h" | "12h" | "daily" | "weekly";
  notify: boolean;
  // A source that is off shows nothing in the Store (catalog, installed
  // apps, updates) and its updates are not looked for.
  showFlatpak: boolean;
  showSnap: boolean;
  showAppimage: boolean;
  // Sections of the home page.
  showFeatured: boolean;
  showPopular: boolean;
  showTrending: boolean;
  // The Drivers section of the sidebar.
  showDrivers: boolean;
  // Look for firmware updates (fwupd) at all; off when the LVFS is down.
  firmwareUpdates: boolean;
}

const DEFAULTS: Settings = {
  flatpakScope: "user",
  autoCheck: true,
  frequency: "daily",
  notify: true,
  showFlatpak: true,
  showSnap: true,
  showAppimage: true,
  showFeatured: true,
  showPopular: true,
  showTrending: true,
  // Off until Drivers is finished.
  showDrivers: false,
  firmwareUpdates: true,
};

const STORAGE_KEY = "soplos-store-settings";

function load(): Settings {
  try {
    return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}") };
  } catch {
    return { ...DEFAULTS };
  }
}

export const settings = writable<Settings>(load());

let autoCheckBefore: boolean | null = null;

settings.subscribe((value) => {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  } catch {
    // Storage unavailable: changes last for this session only.
  }
  // The tray notifier is another process: it reads this copy on disk.
  invoke("save_settings", { settings: value }).catch((error) => console.error("Failed to save settings", error));
  // Turning automatic checks back on starts the notifier right away
  // instead of waiting for the next login (it exits if already running).
  if (value.autoCheck && autoCheckBefore === false) {
    invoke("start_tray").catch((error) => console.error("Failed to start the tray notifier", error));
  }
  autoCheckBefore = value.autoCheck;
});
