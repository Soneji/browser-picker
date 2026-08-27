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
        use embed_manifest::manifest::DpiAwareness;
        use embed_manifest::{embed_manifest, new_manifest};
        // Unaware => Windows scales the whole window to the correct physical size
        // on high-DPI displays (mildly soft, never tiny). Common Controls v6 is
        // included by default so buttons/lists are themed. Crisp per-monitor DPI
        // is a roadmap item.
        embed_manifest(new_manifest("Soneji.BrowserPicker").dpi_awareness(DpiAwareness::Unaware))
            .expect("unable to embed application manifest");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
