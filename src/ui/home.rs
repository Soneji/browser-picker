//! Home / settings window shown when the app is opened directly.

use crate::ui::menu::{self, Menu};
use crate::{browsers, register};

pub fn show() {
    let me = std::env::current_exe().ok();
    let list = browsers::detect(me.as_deref());

    let subtitle = if register::is_registered() {
        "Registered. Use a button below, or Windows Default apps.".to_string()
    } else {
        "Not registered yet — choose \u{201C}Set as default browser\u{201D}.".to_string()
    };

    let info: Vec<String> = if list.is_empty() {
        vec!["No browsers detected.".to_string()]
    } else {
        std::iter::once(format!("Detected {} browser(s):", list.len()))
            .chain(
                list.iter()
                    .enumerate()
                    .map(|(i, b)| format!("    {}.  {}", i + 1, b.name)),
            )
            .collect()
    };

    let menu = Menu {
        title: crate::PRODUCT_NAME.to_string(),
        subtitle,
        info,
        items: vec![
            "Set as default browser".to_string(),
            "Register".to_string(),
            "Unregister".to_string(),
        ],
        footer: "Esc to close".to_string(),
    };

    match menu::run(menu) {
        Some(0) => match register::register() {
            Ok(_) => {
                open_default_apps();
                crate::msg(
                    "Registered.\n\nWindows Default Apps has opened — set Browser Picker for HTTP and HTTPS.",
                );
            }
            Err(e) => crate::msg(&format!("Registration failed:\n{e}")),
        },
        Some(1) => match register::register() {
            Ok(_) => crate::msg("Registered."),
            Err(e) => crate::msg(&format!("Failed:\n{e}")),
        },
        Some(2) => {
            let _ = register::unregister();
            crate::msg("Unregistered.");
        }
        _ => {}
    }
}

fn open_default_apps() {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", "ms-settings:defaultapps"])
        .spawn();
}
