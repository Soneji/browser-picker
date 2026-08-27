//! Persistent user settings: custom browser order and the favourite/default
//! browser. Stored under HKCU so no admin is needed.

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
            Settings { order, favorite }
        }
        Err(_) => Settings::default(),
    }
}

pub fn save(order: &[String], favorite: Option<&str>) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((k, _)) = hkcu.create_subkey(KEY) {
        let _ = k.set_value("Order", &order.join("|"));
        let _ = k.set_value("Favorite", &favorite.unwrap_or("").to_string());
    }
}

/// Reorder `list` by the saved order (browsers not in the saved order keep their
/// original relative position, after the known ones) and return the favourite's
/// index in the reordered list.
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
