//! A small, modern, GPU-free menu window drawn directly with GDI.
//!
//! This deliberately uses no graphics adapter (no OpenGL/DirectX), so it works
//! on any Windows — including Remote Desktop sessions and VMs with no GPU. It is
//! double-buffered (draw to a memory DC, then blit) so there is no flicker, and
//! fully custom-painted for a flat dark look rather than the default Win32 chrome.

use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use winapi::ctypes::c_void;
use winapi::shared::minwindef::{BOOL, FALSE, LPARAM, LRESULT, TRUE, UINT, WPARAM};
use winapi::shared::windef::{HDC, HFONT, HGDIOBJ, HWND, RECT};
use winapi::um::dwmapi::DwmSetWindowAttribute;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen, CreateSolidBrush,
    DeleteDC, DeleteObject, RoundRect, SelectObject, SetBkMode, SetTextColor, CLEARTYPE_QUALITY,
    CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, FF_DONTCARE, OUT_DEFAULT_PRECIS, PS_SOLID,
    SRCCOPY, TRANSPARENT,
};
use winapi::um::winuser::{
    AdjustWindowRectEx, BeginPaint, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    DrawTextW, EndPaint, FillRect, GetClientRect, GetMessageW, GetSystemMetrics, GetWindowLongPtrW,
    InvalidateRect, LoadCursorW, PostQuitMessage, RegisterClassW, SetForegroundWindow,
    SetWindowLongPtrW, ShowWindow, TrackMouseEvent, TranslateMessage, UpdateWindow, CREATESTRUCTW,
    CS_HREDRAW, CS_VREDRAW, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
    GWLP_USERDATA, IDC_ARROW, MSG, PAINTSTRUCT, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW, TME_LEAVE,
    TRACKMOUSEEVENT, VK_ESCAPE, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONUP, WM_MOUSELEAVE,
    WM_MOUSEMOVE, WM_NCCREATE, WM_PAINT, WNDCLASSW, WS_CAPTION, WS_OVERLAPPED, WS_SYSMENU,
};

/// A COLORREF (0x00BBGGRR).
const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

const BG: u32 = rgb(0x1b, 0x1b, 0x1b);
const PANEL: u32 = rgb(0x2a, 0x2a, 0x2a);
const ACCENT: u32 = rgb(0xf3, 0xa2, 0x00);
const TEXT: u32 = rgb(0xea, 0xea, 0xea);
const DIM: u32 = rgb(0xaa, 0xaa, 0xaa);
const DIM2: u32 = rgb(0x80, 0x80, 0x80);
const BLACK: u32 = rgb(0, 0, 0);

const PAD: i32 = 16;
const WIDTH: i32 = 380;
const ROW_H: i32 = 44;
const GAP: i32 = 8;

/// A menu to display. Items are clickable rows; info lines are dim static text.
pub struct Menu {
    pub title: String,
    pub subtitle: String,
    pub info: Vec<String>,
    pub items: Vec<String>,
    pub footer: String,
}

struct State {
    menu: Menu,
    hovered: Option<usize>,
    result: Option<usize>,
    tracking: bool,
    font_title: HFONT,
    font_body: HFONT,
    font_dim: HFONT,
    item_rects: Vec<RECT>,
}

impl Drop for State {
    fn drop(&mut self) {
        unsafe {
            for f in [self.font_title, self.font_body, self.font_dim] {
                if !f.is_null() {
                    DeleteObject(f as HGDIOBJ);
                }
            }
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Truncate a string to `max` chars with an ellipsis.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

unsafe fn make_font(size: i32, weight: i32) -> HFONT {
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

/// Show the menu and block until the user picks an item (Some(index)) or cancels
/// with Esc / the close button (None).
pub fn run(menu: Menu) -> Option<usize> {
    unsafe {
        let hinstance = GetModuleHandleW(null());
        let class_name = wide("BrowserPickerWnd");

        let mut wc: WNDCLASSW = zeroed();
        wc.style = CS_HREDRAW | CS_VREDRAW;
        wc.lpfnWndProc = Some(wnd_proc);
        wc.hInstance = hinstance;
        wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
        wc.hbrBackground = null_mut();
        wc.lpszClassName = class_name.as_ptr();
        RegisterClassW(&wc); // fine if already registered

        // Layout.
        let title_h = if menu.title.is_empty() { 0 } else { 30 };
        let sub_h = if menu.subtitle.is_empty() { 0 } else { 24 };
        let info_h = if menu.info.is_empty() {
            0
        } else {
            menu.info.len() as i32 * 19 + 8
        };
        let items_top = PAD + title_h + sub_h + info_h + 6;
        let n = menu.items.len() as i32;
        let items_h = if n > 0 { n * ROW_H + (n - 1) * GAP } else { 0 };
        let footer_h = if menu.footer.is_empty() { 0 } else { 24 };
        let client_h = items_top + items_h + 10 + footer_h + PAD;

        let mut item_rects = Vec::with_capacity(menu.items.len());
        for i in 0..n {
            let top = items_top + i * (ROW_H + GAP);
            item_rects.push(RECT {
                left: PAD,
                top,
                right: WIDTH - PAD,
                bottom: top + ROW_H,
            });
        }

        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: WIDTH,
            bottom: client_h,
        };
        AdjustWindowRectEx(&mut rc, style, FALSE, 0);
        let win_w = rc.right - rc.left;
        let win_h = rc.bottom - rc.top;
        let x = ((GetSystemMetrics(SM_CXSCREEN) - win_w) / 2).max(0);
        let y = ((GetSystemMetrics(SM_CYSCREEN) - win_h) / 2).max(0);

        let state = Box::new(State {
            menu,
            hovered: None,
            result: None,
            tracking: false,
            font_title: make_font(20, 600),
            font_body: make_font(16, 400),
            font_dim: make_font(13, 400),
            item_rects,
        });
        let state_ptr = Box::into_raw(state);

        let title_w = wide(crate::PRODUCT_NAME);
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title_w.as_ptr(),
            style,
            x,
            y,
            win_w,
            win_h,
            null_mut(),
            null_mut(),
            hinstance,
            state_ptr as *mut c_void,
        );
        if hwnd.is_null() {
            let _ = Box::from_raw(state_ptr);
            return None;
        }

        // Dark title bar (Win10 2004+/Win11); harmless no-op elsewhere.
        let dark: BOOL = TRUE;
        let _ = DwmSetWindowAttribute(
            hwnd,
            20,
            &dark as *const BOOL as *const c_void,
            size_of::<BOOL>() as u32,
        );

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        SetForegroundWindow(hwnd);

        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let state = Box::from_raw(state_ptr);
        state.result
    }
}

fn hit_test(state: &State, x: i32, y: i32) -> Option<usize> {
    for (i, r) in state.item_rects.iter().enumerate() {
        if x >= r.left && x <= r.right && y >= r.top && y <= r.bottom {
            return Some(i);
        }
    }
    None
}

unsafe fn draw_text(hdc: HDC, font: HFONT, color: u32, text: &str, rect: &mut RECT) {
    SelectObject(hdc, font as HGDIOBJ);
    SetTextColor(hdc, color);
    let w = wide(text);
    DrawTextW(
        hdc,
        w.as_ptr(),
        -1,
        rect,
        DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
}

unsafe fn paint(hwnd: HWND, state: &State) {
    let mut ps: PAINTSTRUCT = zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);

    let mut rc: RECT = zeroed();
    GetClientRect(hwnd, &mut rc);
    let w = rc.right;
    let h = rc.bottom;

    // Double buffer.
    let memdc = CreateCompatibleDC(hdc);
    let membmp = CreateCompatibleBitmap(hdc, w, h);
    let oldbmp = SelectObject(memdc, membmp as HGDIOBJ);

    let bg = CreateSolidBrush(BG);
    FillRect(memdc, &rc, bg);
    DeleteObject(bg as HGDIOBJ);
    SetBkMode(memdc, TRANSPARENT as i32);

    let mut y = PAD;
    if !state.menu.title.is_empty() {
        let mut r = line_rect(w, y, 30);
        draw_text(memdc, state.font_title, TEXT, &state.menu.title, &mut r);
        y += 30;
    }
    if !state.menu.subtitle.is_empty() {
        let mut r = line_rect(w, y, 24);
        draw_text(memdc, state.font_dim, DIM, &state.menu.subtitle, &mut r);
        y += 24;
    }
    for line in &state.menu.info {
        let mut r = line_rect(w, y, 19);
        draw_text(memdc, state.font_dim, DIM, line, &mut r);
        y += 19;
    }

    for (i, r) in state.item_rects.iter().enumerate() {
        let hovered = state.hovered == Some(i);
        let fill = if hovered { ACCENT } else { PANEL };
        let brush = CreateSolidBrush(fill);
        let pen = CreatePen(PS_SOLID as i32, 1, fill);
        let ob = SelectObject(memdc, brush as HGDIOBJ);
        let op = SelectObject(memdc, pen as HGDIOBJ);
        RoundRect(memdc, r.left, r.top, r.right, r.bottom, 16, 16);
        SelectObject(memdc, ob);
        SelectObject(memdc, op);
        DeleteObject(brush as HGDIOBJ);
        DeleteObject(pen as HGDIOBJ);

        let color = if hovered { BLACK } else { TEXT };
        let mut tr = RECT {
            left: r.left + 16,
            top: r.top,
            right: r.right - 10,
            bottom: r.bottom,
        };
        draw_text(memdc, state.font_body, color, &state.menu.items[i], &mut tr);
    }

    if !state.menu.footer.is_empty() {
        let mut r = RECT {
            left: PAD,
            top: h - PAD - 20,
            right: w - PAD,
            bottom: h - PAD,
        };
        draw_text(memdc, state.font_dim, DIM2, &state.menu.footer, &mut r);
    }

    BitBlt(hdc, 0, 0, w, h, memdc, 0, 0, SRCCOPY);
    SelectObject(memdc, oldbmp);
    DeleteObject(membmp as HGDIOBJ);
    DeleteDC(memdc);
    EndPaint(hwnd, &ps);
}

fn line_rect(w: i32, y: i32, h: i32) -> RECT {
    RECT {
        left: PAD,
        top: y,
        right: w - PAD,
        bottom: y + h,
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = lparam as *const CREATESTRUCTW;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize);
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let state = &mut *ptr;

    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            paint(hwnd, state);
            0
        }
        WM_MOUSEMOVE => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            if !state.tracking {
                let mut tme: TRACKMOUSEEVENT = zeroed();
                tme.cbSize = size_of::<TRACKMOUSEEVENT>() as u32;
                tme.dwFlags = TME_LEAVE;
                tme.hwndTrack = hwnd;
                TrackMouseEvent(&mut tme);
                state.tracking = true;
            }
            let h = hit_test(state, x, y);
            if h != state.hovered {
                state.hovered = h;
                InvalidateRect(hwnd, null(), FALSE);
            }
            0
        }
        WM_MOUSELEAVE => {
            state.tracking = false;
            if state.hovered.is_some() {
                state.hovered = None;
                InvalidateRect(hwnd, null(), FALSE);
            }
            0
        }
        WM_LBUTTONUP => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            if let Some(i) = hit_test(state, x, y) {
                state.result = Some(i);
                DestroyWindow(hwnd);
            }
            0
        }
        WM_KEYDOWN => {
            let vk = wparam as i32;
            if vk == VK_ESCAPE {
                DestroyWindow(hwnd);
            } else {
                let idx = if (0x31..=0x39).contains(&vk) {
                    Some((vk - 0x31) as usize) // '1'..'9'
                } else if (0x61..=0x69).contains(&vk) {
                    Some((vk - 0x61) as usize) // numpad 1..9
                } else {
                    None
                };
                if let Some(i) = idx {
                    if i < state.menu.items.len() {
                        state.result = Some(i);
                        DestroyWindow(hwnd);
                    }
                }
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
