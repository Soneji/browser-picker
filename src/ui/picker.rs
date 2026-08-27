//! The picker shown when a link is clicked.

use crate::browsers;
use crate::config;
use crate::ui::gdi;
use crate::ui::menu::{self, Menu, MenuItem};

pub fn show(url: String) {
    let me = std::env::current_exe().ok();
    let list = browsers::detect(me.as_deref());
    if list.is_empty() {
        crate::msg("No browsers were detected on this system.");
        return;
    }

    let settings = config::load();
    let (list, fav_idx) = config::apply(list, &settings);

    let items: Vec<MenuItem> = list
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let label = if i < 9 {
                format!("{}      {}", i + 1, b.name)
            } else {
                format!("       {}", b.name)
            };
            let icon = unsafe { gdi::extract_icon(&b.exe) };
            MenuItem {
                label,
                icon,
                favorite: Some(i) == fav_idx,
            }
        })
        .collect();

    let footer = if fav_idx.is_some() {
        "Enter = default   ·   1–9 to pick   ·   Esc".to_string()
    } else {
        "1–9 to pick   ·   Esc to cancel".to_string()
    };

    let menu = Menu {
        title: "Open link in…".to_string(),
        subtitle: gdi::truncate(&url, 52),
        info: Vec::new(),
        items,
        footer,
        default: fav_idx,
    };

    if let Some(i) = menu::run(menu) {
        let _ = browsers::launch(&list[i], &url);
    }
}
