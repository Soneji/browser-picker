pub mod home;
pub mod picker;

/// Top-left position that centers a window of the given outer size on the
/// primary monitor.
pub fn centered_position(w: i32, h: i32) -> (i32, i32) {
    use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    let (sw, sh) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    (((sw - w) / 2).max(0), ((sh - h) / 2).max(0))
}
