//! Persistent user settings: custom browser order, favourite/default, and
//! per-browser shortcut letters. Stored under HKCU so no admin is needed.

use std::collections::{HashMap, HashSet};

use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

use crate::browsers::Browser;

const KEY: &str = r"Software\BrowserPicker";

#[derive(Default)]
pub struct Settings {
    /// Browser ids (lowercased exe paths) in the user's preferred order.
    pub order: Vec<String>,
    /// Favourite/default browser id, if set.
    pub favorite: Option<String>,
    /// User-configured shortcut letters, keyed by browser id.
    pub letters: HashMap<String, char>,
}

pub fn load() -> Settings {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.open_subkey(KEY) {
        Ok(k) => {
            let order = k
                .get_value::<String, _>("Order")
                .unwrap_or_default()
                .split('|')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            let favorite = k
                .get_value::<String, _>("Favorite")
                .ok()
                .filter(|s| !s.is_empty());
            let letters = parse_letters(&k.get_value::<String, _>("Letters").unwrap_or_default());
            Settings {
                order,
                favorite,
                letters,
            }
        }
        Err(_) => Settings::default(),
    }
}

// Letters are stored as "id=L|id2=M|..." — exe paths never contain '=' or '|'.
fn parse_letters(s: &str) -> HashMap<String, char> {
    let mut m = HashMap::new();
    for pair in s.split('|').filter(|p| !p.is_empty()) {
        if let Some((id, letter)) = pair.rsplit_once('=') {
            if let Some(c) = letter.chars().next() {
                if c.is_ascii_alphabetic() {
                    m.insert(id.to_string(), c.to_ascii_uppercase());
                }
            }
        }
    }
    m
}

fn serialize_letters(m: &HashMap<String, char>) -> String {
    m.iter()
        .map(|(id, c)| format!("{id}={c}"))
        .collect::<Vec<_>>()
        .join("|")
}

pub fn save(order: &[String], favorite: Option<&str>, letters: &HashMap<String, char>) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((k, _)) = hkcu.create_subkey(KEY) {
        let _ = k.set_value("Order", &order.join("|"));
        let _ = k.set_value("Favorite", &favorite.unwrap_or("").to_string());
        let _ = k.set_value("Letters", &serialize_letters(letters));
    }
}

/// Reorder `list` by the saved order and return the favourite's index.
pub fn apply(mut list: Vec<Browser>, s: &Settings) -> (Vec<Browser>, Option<usize>) {
    list.sort_by_key(|b| {
        s.order
            .iter()
            .position(|o| *o == b.id())
            .unwrap_or(usize::MAX)
    });
    let fav = s
        .favorite
        .as_ref()
        .and_then(|f| list.iter().position(|b| b.id() == *f));
    (list, fav)
}

/// Resolve the effective shortcut letter for each browser: a user-configured
/// letter wins; otherwise the first alphabetic character of the name is used.
/// Each letter is claimed once (first come) — a browser whose letter is already
/// taken gets none (blank), matching "two starting with the same letter leaves
/// the second without".
pub fn effective_letters(browsers: &[Browser], configured: &HashMap<String, char>) -> Vec<Option<char>> {
    let mut claimed: HashSet<char> = HashSet::new();
    let mut out = vec![None; browsers.len()];

    // Pass 1: honour configured letters (they take priority over auto).
    for (i, b) in browsers.iter().enumerate() {
        if let Some(&c) = configured.get(&b.id()) {
            if c.is_ascii_alphabetic() && claimed.insert(c) {
                out[i] = Some(c);
            }
        }
    }
    // Pass 2: auto-assign the first letter of the name where still free.
    for (i, b) in browsers.iter().enumerate() {
        if out[i].is_some() {
            continue;
        }
        if let Some(c) = b.name.chars().find(|c| c.is_ascii_alphabetic()) {
            let c = c.to_ascii_uppercase();
            if claimed.insert(c) {
                out[i] = Some(c);
            }
        }
    }
    out
}
