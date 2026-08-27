//! The picker shown when a link is clicked.

use crate::browsers;
use crate::ui::menu::{self, Menu};

pub fn show(url: String) {
    let me = std::env::current_exe().ok();
    let list = browsers::detect(me.as_deref());
    if list.is_empty() {
        crate::msg("No browsers were detected on this system.");
        return;
    }

    let items: Vec<String> = list
        .iter()
        .enumerate()
        .map(|(i, b)| {
            if i < 9 {
                format!("{}      {}", i + 1, b.name)
            } else {
                format!("       {}", b.name)
            }
        })
        .collect();

    let menu = Menu {
        title: "Open link in…".to_string(),
        subtitle: menu::truncate(&url, 52),
        info: Vec::new(),
        items,
        footer: "1–9 to pick   ·   Esc to cancel".to_string(),
    };

    if let Some(i) = menu::run(menu) {
        let _ = browsers::launch(&list[i], &url);
    }
}
