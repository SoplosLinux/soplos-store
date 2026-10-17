import { writable } from "svelte/store";

export type Theme = "light" | "dark";

const STORAGE_KEY = "soplos-store-theme";

function readStoredTheme(): Theme {
  try {
    return localStorage.getItem(STORAGE_KEY) === "light" ? "light" : "dark";
  } catch {
    return "dark";
  }
}

function applyTheme(value: Theme) {
  document.documentElement.classList.toggle("light", value === "light");
  try {
    localStorage.setItem(STORAGE_KEY, value);
  } catch {
    // Storage unavailable: the theme still applies for this session.
  }
}

// The saved value is read before the store exists, so the first
// subscription call cannot overwrite it with the default.
export const theme = writable<Theme>(readStoredTheme());
theme.subscribe(applyTheme);

export function toggleTheme() {
  theme.update((current) => (current === "dark" ? "light" : "dark"));
}
