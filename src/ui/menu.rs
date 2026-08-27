//! The picker: a frameless, top-most popup that appears at the mouse cursor with
//! the favourite/default row under the pointer (so a double-click on a link opens
//! the default). Position is clamped to the cursor's monitor work area, so a click
//! near a screen edge slides the popup on-screen instead of overflowing. Closes on
//! Esc or when it loses focus (click-away). GPU-free GDI.

use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use winapi::ctypes::c_void;
use winapi::shared::minwindef::{FALSE, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HFONT, HGDIOBJ, HICON, HWND, POINT, RECT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreatePen, CreateSolidBrush, DeleteDC,
    DeleteObject, RoundRect, SelectObject, SetBkMode, PS_SOLID, SRCCOPY, TRANSPARENT,
};
use winapi::um::winuser::{
    BeginPaint, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow, DispatchMessageW,
    EndPaint, FillRect, FrameRect, GetClientRect, GetCursorPos, GetMessageW, GetMonitorInfoW,
    GetWindowLongPtrW, InvalidateRect, LoadCursorW, MonitorFromPoint, PostQuitMessage,
    RegisterClassW, SetForegroundWindow, SetWindowLongPtrW, ShowWindow, TrackMouseEvent,
    TranslateMessage, UpdateWindow, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, IDC_ARROW,
    MONITORINFO, MONITOR_DEFAULTTONEAREST, MSG, PAINTSTRUCT, SW_SHOW, TME_LEAVE, TRACKMOUSEEVENT,
    VK_ESCAPE, VK_RETURN, VK_SPACE, WA_INACTIVE, WM_ACTIVATE, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN,
    WM_LBUTTONUP, WM_MOUSELEAVE, WM_MOUSEMOVE, WM_NCCREATE, WM_PAINT, WNDCLASSW, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};

use crate::ui::gdi::{
    self, ACCENT, BG, BLACK, BORDER, DIM, DIM2, DT_GLYPH, DT_LINE, PANEL, TEXT,
};

const PAD: i32 = 16;
const WIDTH: i32 = 380;
const ROW_H: i32 = 46;
const GAP: i32 = 8;
const ICON: i32 = 24;
const MARGIN: i32 = 8;

pub struct MenuItem {
    pub label: String,
    pub icon: Option<HICON>,
    pub favorite: bool,
}

pub struct Menu {
    pub title: String,
    pub subtitle: String,
    pub info: Vec<String>,
    pub items: Vec<MenuItem>,
    pub footer: String,
    /// Index launched on Enter/Space and anchored under the cursor (the default).
    pub default: Option<usize>,
}

struct State {
    menu: Menu,
    hovered: Option<usize>,
    result: Option<usize>,
    tracking: bool,
    shown: bool,
    font_title: HFONT,
    font_body: HFONT,
    font_dim: HFONT,
    item_rects: Vec<RECT>,
    has_icons: bool,
}

impl Drop for State {
    fn drop(&mut self) {
        unsafe {
            for it in &self.menu.items {
                if let Some(ic) = it.icon {
                    DestroyIcon(ic);
                }
            }
            for f in [self.font_title, self.font_body, self.font_dim] {
                if !f.is_null() {
                    DeleteObject(f as HGDIOBJ);
                }
            }
        }
    }
}

pub fn run(menu: Menu) -> Option<usize> {
    unsafe {
        let hinstance = GetModuleHandleW(null());
        let class_name = gdi::wide("BrowserPickerWnd");
        let mut wc: WNDCLASSW = zeroed();
        wc.style = CS_HREDRAW | CS_VREDRAW;
        wc.lpfnWndProc = Some(wnd_proc);
        wc.hInstance = hinstance;
        wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
        wc.lpszClassName = class_name.as_ptr();
        RegisterClassW(&wc);

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
        let has_icons = menu.items.iter().any(|it| it.icon.is_some());

        // Anchor the favourite/default row (or the first row) under the cursor.
        let anchor_row = menu.default.unwrap_or(0).min(menu.items.len().saturating_sub(1));
        let anchor_y = item_rects[anchor_row].top + ROW_H / 2;

        // Cursor position and its monitor's work area.
        let mut pt: POINT = zeroed();
        GetCursorPos(&mut pt);
        let mut mi: MONITORINFO = zeroed();
        mi.cbSize = size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST), &mut mi);
        let work = mi.rcWork;

        // Centre horizontally on the cursor; put the anchor row under it; clamp.
        let hi_left = (work.right - WIDTH - MARGIN).max(work.left + MARGIN);
        let x = (pt.x - WIDTH / 2).max(work.left + MARGIN).min(hi_left);
        let hi_top = (work.bottom - client_h - MARGIN).max(work.top + MARGIN);
        let y = (pt.y - anchor_y).max(work.top + MARGIN).min(hi_top);

        let state = Box::new(State {
            menu,
            hovered: None,
            result: None,
            tracking: false,
            shown: false,
            font_title: gdi::make_font(20, 600),
            font_body: gdi::make_font(16, 400),
            font_dim: gdi::make_font(13, 400),
            item_rects,
            has_icons,
        });
        let state_ptr = Box::into_raw(state);

        let title_w = gdi::wide(crate::PRODUCT_NAME);
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            class_name.as_ptr(),
            title_w.as_ptr(),
            WS_POPUP,
            x,
            y,
            WIDTH,
            client_h,
            null_mut(),
            null_mut(),
            hinstance,
            state_ptr as *mut c_void,
        );
        if hwnd.is_null() {
            let _ = Box::from_raw(state_ptr);
            return None;
        }

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        SetForegroundWindow(hwnd);
        (*state_ptr).shown = true;

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

unsafe fn paint(hwnd: HWND, state: &State) {
    let mut ps: PAINTSTRUCT = zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mut rc: RECT = zeroed();
    GetClientRect(hwnd, &mut rc);
    let w = rc.right;
    let h = rc.bottom;

    let memdc = CreateCompatibleDC(hdc);
    let membmp = CreateCompatibleBitmap(hdc, w, h);
    let oldbmp = SelectObject(memdc, membmp as HGDIOBJ);

    let bgb = CreateSolidBrush(BG);
    FillRect(memdc, &rc, bgb);
    DeleteObject(bgb as HGDIOBJ);
    SetBkMode(memdc, TRANSPARENT as i32);

    let mut y = PAD;
    if !state.menu.title.is_empty() {
        let mut r = line_rect(w, y, 30);
        gdi::draw_text(memdc, state.font_title, TEXT, &state.menu.title, &mut r, DT_LINE);
        y += 30;
    }
    if !state.menu.subtitle.is_empty() {
        let mut r = line_rect(w, y, 24);
        gdi::draw_text(memdc, state.font_dim, DIM, &state.menu.subtitle, &mut r, DT_LINE);
        y += 24;
    }
    for line in &state.menu.info {
        let mut r = line_rect(w, y, 19);
        gdi::draw_text(memdc, state.font_dim, DIM, line, &mut r, DT_LINE);
        y += 19;
    }

    let text_left_off = if state.has_icons { 12 + ICON + 12 } else { 16 };
    for (i, r) in state.item_rects.iter().enumerate() {
        let it = &state.menu.items[i];
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

        if let Some(ic) = it.icon {
            gdi::draw_icon(memdc, r.left + 12, r.top + (ROW_H - ICON) / 2, ICON, ic);
        }
        let color = if hovered { BLACK } else { TEXT };
        let mut tr = RECT {
            left: r.left + text_left_off,
            top: r.top,
            right: r.right - 36,
            bottom: r.bottom,
        };
        gdi::draw_text(memdc, state.font_body, color, &it.label, &mut tr, DT_LINE);

        if it.favorite {
            let mut sr = RECT {
                left: r.right - 34,
                top: r.top,
                right: r.right - 8,
                bottom: r.bottom,
            };
            let scol = if hovered { BLACK } else { ACCENT };
            gdi::draw_text(memdc, state.font_body, scol, "★", &mut sr, DT_GLYPH);
        }
    }

    if !state.menu.footer.is_empty() {
        let mut r = RECT {
            left: PAD,
            top: h - PAD - 20,
            right: w - PAD,
            bottom: h - PAD,
        };
        gdi::draw_text(memdc, state.font_dim, DIM2, &state.menu.footer, &mut r, DT_LINE);
    }

    // Thin border around the frameless popup.
    let border = CreateSolidBrush(BORDER);
    FrameRect(memdc, &rc, border);
    DeleteObject(border as HGDIOBJ);

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

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
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
        WM_ACTIVATE => {
            // Close when the popup loses focus (click-away), once fully shown.
            if (wparam & 0xFFFF) == WA_INACTIVE as usize && state.shown {
                DestroyWindow(hwnd);
            }
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
            } else if vk == VK_RETURN || vk == VK_SPACE {
                if let Some(i) = state.menu.default {
                    state.result = Some(i);
                    DestroyWindow(hwnd);
                }
            } else {
                let idx = if (0x31..=0x39).contains(&vk) {
                    Some((vk - 0x31) as usize)
                } else if (0x61..=0x69).contains(&vk) {
                    Some((vk - 0x61) as usize)
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
