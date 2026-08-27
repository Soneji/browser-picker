#![windows_subsystem = "windows"]
//! Browser Picker — a tiny, native browser chooser for Windows.
//!
//! When set as the default browser, Windows launches this exe with the clicked
//! URL as its argument. We detect every installed browser from the registry and
//! show a small, modern window (egui) to pick one. There is no background
//! process — the exe only runs while you're choosing, then exits.

mod browsers;
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
