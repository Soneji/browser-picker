//! Register / unregister Browser Picker as a selectable Windows browser.
//!
//! Everything is written under HKCU, so no administrator rights are needed
//! (this mirrors how per-user Chrome/Edge installs register themselves).
//!
//! Windows 10/11 does not allow an app to make itself the *default* browser
//! programmatically — the user confirms that once in Settings. This registration
//! is what makes Browser Picker *appear* as a valid choice for http/https.

use std::io;

use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};
use winreg::RegKey;

use crate::{PRODUCT_NAME, PROG_ID, PROG_KEY};

fn exe_string() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn register() -> io::Result<()> {
    let exe = exe_string();
    let command = format!("\"{exe}\" \"%1\"");
    let icon = format!("{exe},0");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. Clients\StartMenuInternet\BrowserPicker
    let base = format!(r"Software\Clients\StartMenuInternet\{PROG_KEY}");
    hkcu.create_subkey(&base)?.0.set_value("", &PRODUCT_NAME.to_string())?;
    hkcu.create_subkey(format!(r"{base}\DefaultIcon"))?.0.set_value("", &icon)?;
    hkcu.create_subkey(format!(r"{base}\shell\open\command"))?
        .0
        .set_value("", &command)?;

    // 2. Capabilities (this is what surfaces us in "Default apps")
    let cap = format!(r"{base}\Capabilities");
    let capk = hkcu.create_subkey(&cap)?.0;
    capk.set_value("ApplicationName", &PRODUCT_NAME.to_string())?;
    capk.set_value(
        "ApplicationDescription",
        &"Choose which browser opens each link.".to_string(),
    )?;
    capk.set_value("ApplicationIcon", &icon)?;

    let urls = hkcu.create_subkey(format!(r"{cap}\URLAssociations"))?.0;
    urls.set_value("http", &PROG_ID.to_string())?;
    urls.set_value("https", &PROG_ID.to_string())?;

    hkcu.create_subkey(format!(r"{cap}\StartMenu"))?
        .0
        .set_value("StartMenuInternet", &PROG_KEY.to_string())?;

    // 3. ProgID that the URLAssociations point at
    let progid = format!(r"Software\Classes\{PROG_ID}");
    hkcu.create_subkey(&progid)?.0.set_value("", &PRODUCT_NAME.to_string())?;
    hkcu.create_subkey(format!(r"{progid}\DefaultIcon"))?.0.set_value("", &icon)?;
    hkcu.create_subkey(format!(r"{progid}\shell\open\command"))?
        .0
        .set_value("", &command)?;

    // 4. RegisteredApplications -> Capabilities
    hkcu.create_subkey(r"Software\RegisteredApplications")?
        .0
        .set_value(
            PRODUCT_NAME,
            &format!(r"Software\Clients\StartMenuInternet\{PROG_KEY}\Capabilities"),
        )?;

    Ok(())
}

pub fn unregister() -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(format!(r"Software\Clients\StartMenuInternet\{PROG_KEY}"));
    let _ = hkcu.delete_subkey_all(format!(r"Software\Classes\{PROG_ID}"));
    if let Ok(ra) =
        hkcu.open_subkey_with_flags(r"Software\RegisteredApplications", KEY_ALL_ACCESS)
    {
        let _ = ra.delete_value(PRODUCT_NAME);
    }
    Ok(())
}

pub fn is_registered() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(format!(
            r"Software\Clients\StartMenuInternet\{PROG_KEY}\Capabilities"
        ))
        .is_ok()
}
