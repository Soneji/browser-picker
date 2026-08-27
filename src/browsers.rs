//! Browser detection and launching.
//!
//! Detection reads `Clients\StartMenuInternet` from three registry locations so
//! per-user installs, machine-wide installs, and 32-bit browsers on 64-bit
//! Windows are all found:
//!   * `HKCU\SOFTWARE\Clients\StartMenuInternet`
//!   * `HKLM\SOFTWARE\Clients\StartMenuInternet`
//!   * `HKLM\SOFTWARE\WOW6432Node\Clients\StartMenuInternet`
//!
//! Results are de-duplicated by executable path. No allow-list is applied — any
//! app that registers itself as an internet client is offered. This is the key
//! difference from mac-heritage pickers that only know a fixed set of browsers
//! or only read one hive and therefore miss per-user Chrome/Edge/Brave installs.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
use winreg::RegKey;

#[derive(Clone, Debug)]
pub struct Browser {
    /// `StartMenuInternet` subkey name (e.g. "Google Chrome").
    #[allow(dead_code)]
    pub key: String,
    /// Friendly display name.
    pub name: String,
    /// Raw `shell\open\command` template (kept for reference / fallback).
    #[allow(dead_code)]
    pub command: String,
    /// Parsed executable path.
    pub exe: PathBuf,
    /// `DefaultIcon` value ("path,index") — reserved for future icon support.
    #[allow(dead_code)]
    pub icon: String,
}

const START_MENU: &str = r"SOFTWARE\Clients\StartMenuInternet";
const START_MENU_WOW: &str = r"SOFTWARE\WOW6432Node\Clients\StartMenuInternet";

/// Detect installed browsers. `exclude_exe` (usually our own path) is filtered
/// out so the picker never lists itself.
pub fn detect(exclude_exe: Option<&Path>) -> Vec<Browser> {
    let sources = [
        (HKEY_CURRENT_USER, START_MENU),
        (HKEY_LOCAL_MACHINE, START_MENU),
        (HKEY_LOCAL_MACHINE, START_MENU_WOW),
    ];

    let mut found: Vec<Browser> = Vec::new();
    for (hive, path) in sources {
        let root = RegKey::predef(hive);
        if let Ok(smi) = root.open_subkey_with_flags(path, KEY_READ | KEY_WOW64_64KEY) {
            for key_name in smi.enum_keys().flatten() {
                if key_name.eq_ignore_ascii_case(crate::PROG_KEY) {
                    continue; // never list ourselves
                }
                if let Ok(b) = read_browser(&smi, &key_name) {
                    found.push(b);
                }
            }
        }
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<Browser> = Vec::new();
    for b in found {
        if b.exe.as_os_str().is_empty() {
            continue;
        }
        if let Some(ex) = exclude_exe {
            if same_path(&b.exe, ex) {
                continue;
            }
        }
        let dedupe = b.exe.to_string_lossy().to_lowercase();
        if seen.insert(dedupe) {
            out.push(b);
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

fn read_browser(smi: &RegKey, key_name: &str) -> std::io::Result<Browser> {
    let bkey = smi.open_subkey(key_name)?;

    // Prefer Capabilities\ApplicationName (nicer, localized) then the key's
    // default value, then finally the raw subkey name.
    let mut name: String = bkey
        .get_value::<String, _>("")
        .unwrap_or_else(|_| key_name.to_string());
    if let Ok(cap) = bkey.open_subkey("Capabilities") {
        if let Ok(app_name) = cap.get_value::<String, _>("ApplicationName") {
            if !app_name.trim().is_empty() {
                name = app_name;
            }
        }
    }
    if name.trim().is_empty() {
        name = key_name.to_string();
    }

    let command: String = bkey
        .open_subkey(r"shell\open\command")?
        .get_value::<String, _>("")?;
    let exe = parse_exe(&command);

    let icon: String = bkey
        .open_subkey("DefaultIcon")
        .and_then(|k| k.get_value::<String, _>(""))
        .unwrap_or_default();

    Ok(Browser {
        key: key_name.to_string(),
        name,
        command,
        exe,
        icon,
    })
}

/// Extract the executable path from a `shell\open\command` template such as
/// `"C:\...\chrome.exe" -- "%1"` or `C:\...\browser.exe %1`.
fn parse_exe(command: &str) -> PathBuf {
    let cmd = command.trim();
    if let Some(rest) = cmd.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return PathBuf::from(&rest[..end]);
        }
    }
    let first = cmd.split_whitespace().next().unwrap_or("");
    PathBuf::from(first)
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

/// Launch `browser` with `url`. Mainstream browsers all accept the URL as a
/// single argument, which avoids fragile re-quoting of the registry command
/// template (e.g. Chrome/Edge/Firefox/Brave/Opera/Vivaldi all handle `exe <url>`).
pub fn launch(browser: &Browser, url: &str) -> std::io::Result<()> {
    // The picker window currently owns the foreground; grant that right to the
    // browser we're about to launch (or its already-running instance) so its
    // window comes to the front instead of opening behind other windows.
    unsafe {
        // ASFW_ANY = (DWORD)-1 : permit any process to set the foreground window.
        winapi::um::winuser::AllowSetForegroundWindow(u32::MAX);
    }
    Command::new(&browser.exe).arg(url).spawn().map(|_| ())
}
