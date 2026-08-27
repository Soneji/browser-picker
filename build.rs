// Embeds a Windows application manifest so the app gets:
//   * Common Controls v6  -> themed (not Win95-looking) buttons/lists
//   * PerMonitorV2 DPI awareness -> crisp on high-DPI displays
//
// Only done for the MSVC target (the real release build in CI). The manifest is
// applied at link time, so the local `cargo check --target ...-gnu` type-check
// on Linux skips it entirely and needs no external tools (windres, etc.).
fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows") && target.contains("msvc") {
        use embed_manifest::{embed_manifest, new_manifest};
        embed_manifest(new_manifest("Soneji.BrowserPicker"))
            .expect("unable to embed application manifest");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
