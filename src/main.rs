#![windows_subsystem = "windows"]
//! Browser Picker — a tiny, native browser chooser for Windows.
//!
//! When set as the default browser, Windows launches this exe with the clicked
//! URL as its argument. We detect every installed browser from the registry and
//! show a small, modern window (egui) to pick one. There is no background
//! process — the exe only runs while you're choosing, then exits.

mod browsers;
mod config;
mod register;
mod ui;

/// Display name shown to the user and in Windows "Default apps".
pub const PRODUCT_NAME: &str = "Browser Picker";
/// `StartMenuInternet` subkey name and `RegisteredApplications` value name.
pub const PROG_KEY: &str = "BrowserPicker";
/// ProgID used for http/https URL associations.
pub const PROG_ID: &str = "BrowserPicker.Url";

const HELP: &str = "Browser Picker — choose which browser opens each link.\n\n\
Usage:\n\
  browser-picker.exe <url>          Show the picker for a URL\n\
  browser-picker.exe                Open the home / settings window\n\
  browser-picker.exe --register     Register as a selectable browser\n\
  browser-picker.exe --unregister   Remove the registration\n\
  browser-picker.exe --list         List the browsers that were detected\n\
  browser-picker.exe --help         Show this help";

/// Native message box for the CLI-style flags (GUI-subsystem app: no console).
pub fn msg(text: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }
    let body = wide(text);
    let title = wide(PRODUCT_NAME);
    unsafe {
        winapi::um::winuser::MessageBoxW(
            std::ptr::null_mut(),
            body.as_ptr(),
            title.as_ptr(),
            winapi::um::winuser::MB_OK,
        );
    }
}

/// Turn any panic into a visible, copyable dialog + a log file, so a crash
/// (e.g. the graphics renderer failing to start) is diagnosable, not silent.
fn install_crash_handler() {
    std::panic::set_hook(Box::new(|info| {
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown error".to_string());
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown location".to_string());
        let text =
            format!("Browser Picker hit an error and had to close.\n\n{payload}\n\nAt: {location}");
        let log = std::env::temp_dir().join("browser-picker-crash.log");
        let _ = std::fs::write(&log, &text);
        msg(&format!("{text}\n\nA copy was saved to:\n{}", log.display()));
    }));
}

fn list_text() -> String {
    let me = std::env::current_exe().ok();
    let found = browsers::detect(me.as_deref());
    if found.is_empty() {
        return "No browsers were detected.".to_string();
    }
    let mut s = format!("Detected {} browser(s):\n\n", found.len());
    for (i, b) in found.iter().enumerate() {
        s.push_str(&format!("{}.  {}\n     {}\n", i + 1, b.name, b.exe.display()));
    }
    s
}

fn main() {
    install_crash_handler();

    let args: Vec<String> = std::env::args().skip(1).collect();

    // First recognised flag wins; first non-flag argument is treated as the URL.
    let mut url: Option<String> = None;
    let mut flag: Option<String> = None;
    for a in &args {
        if a.starts_with("--") || a == "-h" {
            if flag.is_none() {
                flag = Some(a.clone());
            }
        } else if url.is_none() {
            url = Some(a.clone());
        }
    }

    match flag.as_deref() {
        Some("--register") => match register::register() {
            Ok(_) => msg(&format!(
                "{} is now registered.\n\nOpen Settings ▸ Apps ▸ Default apps, find \"{}\", \
                 and set it for HTTP and HTTPS.",
                PRODUCT_NAME, PRODUCT_NAME
            )),
            Err(e) => msg(&format!("Registration failed:\n{e}")),
        },
        Some("--unregister") => {
            let _ = register::unregister();
            msg(&format!("{PRODUCT_NAME} has been unregistered."));
        }
        Some("--list") => msg(&list_text()),
        Some("--help") | Some("-h") => msg(HELP),
        Some(_) | None => match url {
            Some(u) => ui::picker::show(u),
            None => ui::home::show(),
        },
    }
}
