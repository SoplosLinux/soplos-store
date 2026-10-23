import { derived, get, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

export type Theme = "light" | "dark";
/** What the user chose: a theme, or "auto" to follow the desktop. */
export type ThemeChoice = "auto" | Theme;

const STORAGE_KEY = "soplos-store-theme-choice";
// How often the desktop is asked again while the window follows it.
const FOLLOW_EVERY_MS = 4000;

function readStoredChoice(): ThemeChoice {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return value === "light" || value === "dark" ? value : "auto";
  } catch {
    return "auto";
  }
}

function store(value: ThemeChoice) {
  try {
    localStorage.setItem(STORAGE_KEY, value);
  } catch {
    // Storage unavailable: the choice still applies for this session.
  }
}

// The saved value is read before the store exists, so the first
// subscription call cannot overwrite it with the default.
export const themeChoice = writable<ThemeChoice>(readStoredChoice());
themeChoice.subscribe(store);

// The theme of the desktop, until it is known the dark one the app always had.
const systemTheme = writable<Theme>("dark");

/** The theme in use: the chosen one, or the desktop's when the choice is "auto". */
export const theme = derived([themeChoice, systemTheme], ([choice, system]): Theme => (choice === "auto" ? system : choice));
theme.subscribe((value) => document.documentElement.classList.toggle("light", value === "light"));

async function readSystemTheme() {
  try {
    const found = await invoke<string | null>("get_system_theme");
    if (found === "light" || found === "dark") systemTheme.set(found);
  } catch {
    // Not asked yet or no answer: the last known theme stays.
  }
}

let following: ReturnType<typeof setInterval> | null = null;

/** Starts following the desktop while the choice is "auto"; call once. */
export function followSystemTheme() {
  if (following) return;
  void readSystemTheme();
  following = setInterval(() => {
    if (get(themeChoice) === "auto") void readSystemTheme();
  }, FOLLOW_EVERY_MS);
  // A theme changed in the settings of the desktop is seen as soon as the
  // window is looked at again.
  window.addEventListener("focus", () => get(themeChoice) === "auto" && void readSystemTheme());
}

/** The button of the sidebar: it chooses the other theme, and the choice stays. */
export function toggleTheme() {
  themeChoice.set(get(theme) === "dark" ? "light" : "dark");
}
