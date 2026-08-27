//! Shared GDI helpers for the (GPU-free) picker and settings windows.

use std::path::Path;
use std::ptr::null_mut;

use winapi::shared::windef::{HDC, HFONT, HGDIOBJ, HICON, RECT};
use winapi::um::shellapi::ExtractIconExW;
use winapi::um::wingdi::{
    CreateFontW, SelectObject, SetTextColor, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS,
    DEFAULT_CHARSET, DEFAULT_PITCH, FF_DONTCARE, OUT_DEFAULT_PRECIS,
};
use winapi::um::winuser::{
    DestroyIcon, DrawIconEx, DrawTextW, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX,
    DT_SINGLELINE, DT_VCENTER,
};

/// DrawIconEx flag (DI_IMAGE | DI_MASK); winapi 0.3 doesn't export it.
const DI_NORMAL: u32 = 0x0003;

/// Build a COLORREF (0x00BBGGRR).
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub const BG: u32 = rgb(0x1b, 0x1b, 0x1b);
pub const PANEL: u32 = rgb(0x2a, 0x2a, 0x2a);
pub const PANEL2: u32 = rgb(0x38, 0x38, 0x38);
pub const ACCENT: u32 = rgb(0xf3, 0xa2, 0x00);
pub const TEXT: u32 = rgb(0xea, 0xea, 0xea);
pub const DIM: u32 = rgb(0xaa, 0xaa, 0xaa);
pub const DIM2: u32 = rgb(0x80, 0x80, 0x80);
pub const BLACK: u32 = rgb(0, 0, 0);
pub const BORDER: u32 = rgb(0x55, 0x55, 0x55);

/// A left-aligned, vertically-centred single line with ellipsis.
pub const DT_LINE: u32 = DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_END_ELLIPSIS;
/// A centred single line (for glyph buttons).
pub const DT_GLYPH: u32 = DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX;

pub fn wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

pub unsafe fn make_font(size: i32, weight: i32) -> HFONT {
    let face = wide("Segoe UI");
    CreateFontW(
        -size,
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH | FF_DONTCARE,
        face.as_ptr(),
    )
}

pub unsafe fn draw_text(hdc: HDC, font: HFONT, color: u32, text: &str, rect: &mut RECT, flags: u32) {
    SelectObject(hdc, font as HGDIOBJ);
    SetTextColor(hdc, color);
    let w = wide(text);
    DrawTextW(hdc, w.as_ptr(), -1, rect, flags);
}

/// Extract the first icon from an executable (large icon; falls back to small).
/// The caller owns the returned HICON and must DestroyIcon it.
pub unsafe fn extract_icon(path: &Path) -> Option<HICON> {
    let wpath = wide(&path.to_string_lossy());
    let mut large: HICON = null_mut();
    let mut small: HICON = null_mut();
    ExtractIconExW(wpath.as_ptr(), 0, &mut large, &mut small, 1);
    if !large.is_null() {
        if !small.is_null() {
            DestroyIcon(small);
        }
        Some(large)
    } else if !small.is_null() {
        Some(small)
    } else {
        None
    }
}

pub unsafe fn draw_icon(hdc: HDC, x: i32, y: i32, size: i32, icon: HICON) {
    DrawIconEx(hdc, x, y, icon, size, size, 0, null_mut(), DI_NORMAL);
}
