import { addMessages, init, locale } from "svelte-i18n";
import { writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import en from "./locales/en.json";
import es from "./locales/es.json";
import de from "./locales/de.json";
import fr from "./locales/fr.json";
import it from "./locales/it.json";
import pt from "./locales/pt.json";
import ro from "./locales/ro.json";
import ru from "./locales/ru.json";

export const SUPPORTED_LOCALES = ["en", "es", "de", "fr", "it", "pt", "ro", "ru"] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

export const LOCALE_NAMES: Record<SupportedLocale, string> = {
  en: "English",
  es: "Español",
  de: "Deutsch",
  fr: "Français",
  it: "Italiano",
  pt: "Português",
  ro: "Română",
  ru: "Русский",
};

const STORAGE_KEY = "soplos-store-locale";

addMessages("en", en);
addMessages("es", es);
addMessages("de", de);
addMessages("fr", fr);
addMessages("it", it);
addMessages("pt", pt);
addMessages("ro", ro);
addMessages("ru", ru);

function isSupported(code: string | null): code is SupportedLocale {
  return !!code && (SUPPORTED_LOCALES as readonly string[]).includes(code);
}

// A language chosen in the sidebar wins; otherwise the system language,
// and English when the system language is not one of the eight.
function initialLocale(): SupportedLocale {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isSupported(stored)) return stored;
  } catch {
    // Storage unavailable: fall through to the system language.
  }
  const system = (navigator.language || "en").slice(0, 2).toLowerCase();
  return isSupported(system) ? system : "en";
}

const startLocale = initialLocale();
init({ fallbackLocale: "en", initialLocale: startLocale });

// Raised once the backend knows the new language, so the pages that show
// data written in it (names and descriptions from Debian) load it again.
export const languageChanged = writable(0);

// The backend writes some texts itself (tray, update log, Debian's
// metadata); it has to know which language the user chose.
function tellBackend(code: SupportedLocale) {
  return invoke("set_language", { code }).catch((error) => console.error("Failed to set the language", error));
}

void tellBackend(startLocale);

export async function setLocale(code: SupportedLocale) {
  locale.set(code);
  try {
    localStorage.setItem(STORAGE_KEY, code);
  } catch {
    // Storage unavailable: the language still changes for this session.
  }
  await tellBackend(code);
  languageChanged.update((n) => n + 1);
}
