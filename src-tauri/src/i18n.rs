// Texts written by the backend itself (the tray menu, the update
// notification and the lines of the "check for updates" log). They are not
// written here: they come from the same dictionaries the window uses
// (src/i18n/locales/*.json), embedded in the binary, so there is a single
// place for every translation.
//
// The language is the one chosen in the window, which tells the backend
// (set_language) and which is kept in ~/.config/soplos-store/language so
// the tray notifier, a separate process, speaks the same language. Without
// a choice it falls back to the language of the session.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde_json::Value;

use crate::system;

const LOCALES: [(&str, &str); 8] = [
    ("en", include_str!("../../src/i18n/locales/en.json")),
    ("es", include_str!("../../src/i18n/locales/es.json")),
    ("de", include_str!("../../src/i18n/locales/de.json")),
    ("fr", include_str!("../../src/i18n/locales/fr.json")),
    ("it", include_str!("../../src/i18n/locales/it.json")),
    ("pt", include_str!("../../src/i18n/locales/pt.json")),
    ("ro", include_str!("../../src/i18n/locales/ro.json")),
    ("ru", include_str!("../../src/i18n/locales/ru.json")),
];

static CHOSEN: Mutex<Option<String>> = Mutex::new(None);

fn dictionaries() -> &'static HashMap<&'static str, Value> {
    static DICTIONARIES: OnceLock<HashMap<&'static str, Value>> = OnceLock::new();
    DICTIONARIES.get_or_init(|| LOCALES.iter().filter_map(|(code, json)| Some((*code, serde_json::from_str(json).ok()?))).collect())
}

fn supported(code: &str) -> bool {
    LOCALES.iter().any(|(known, _)| *known == code)
}

fn language_file() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("soplos-store").join("language"))
}

/// The language of the app: the choice made in the window (in this
/// process or saved by it), or the language of the session.
pub fn current() -> String {
    if let Some(code) = CHOSEN.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        return code;
    }
    language_file()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|text| text.trim().to_string())
        .filter(|code| supported(code))
        .unwrap_or_else(system::session_language)
}

/// The window tells the backend which language the user chose. Debian's
/// metadata is read in that language, so it is read again.
#[tauri::command]
pub async fn set_language(code: String) {
    if !supported(&code) {
        return;
    }
    *CHOSEN.lock().unwrap_or_else(|e| e.into_inner()) = Some(code.clone());
    if let Some(path) = language_file() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, code);
    }
    let _ = tauri::async_runtime::spawn_blocking(|| {
        crate::apt::reload();
        crate::apt::invalidate_installed();
    })
    .await;
}

fn lookup(dictionary: &Value, key: &str) -> Option<String> {
    key.split('.').try_fold(dictionary, |node, part| node.get(part))?.as_str().map(String::from)
}

/// The text for a dotted key such as "tray.open", in the language of the
/// app and in English when that language has no such text.
pub fn t(key: &str) -> String {
    let dictionaries = dictionaries();
    let language = current();
    dictionaries
        .get(language.as_str())
        .and_then(|d| lookup(d, key))
        .or_else(|| dictionaries.get("en").and_then(|d| lookup(d, key)))
        .unwrap_or_else(|| key.to_string())
}

/// Like `t`, with "{n}" replaced by a number.
pub fn t_n(key: &str, n: usize) -> String {
    t(key).replace("{n}", &n.to_string())
}
