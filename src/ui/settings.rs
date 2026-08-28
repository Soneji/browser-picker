//! The settings/home window (GPU-free GDI). Reorder browsers, pick a
//! favourite/default, assign shortcut letters, and register the app. Changes
//! persist immediately.

use std::collections::HashMap;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use winapi::ctypes::c_void;
use winapi::shared::minwindef::{BOOL, FALSE, LPARAM, LRESULT, TRUE, UINT, WPARAM};
use winapi::shared::windef::{HFONT, HGDIOBJ, HICON, HWND, RECT};
use winapi::um::dwmapi::DwmSetWindowAttribute;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreatePen, CreateSolidBrush, DeleteDC,
    DeleteObject, RoundRect, SelectObject, SetBkMode, PS_SOLID, SRCCOPY, TRANSPARENT,
};
use winapi::um::winuser::{
    AdjustWindowRectEx, BeginPaint, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow,
    DispatchMessageW, EndPaint, FillRect, GetClientRect, GetMessageW, GetSystemMetrics,
    GetWindowLongPtrW, InvalidateRect, LoadCursorW, PostQuitMessage, RegisterClassW,
    SetForegroundWindow, SetWindowLongPtrW, ShowWindow, TrackMouseEvent, TranslateMessage,
    UpdateWindow, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, IDC_ARROW,
    MSG, PAINTSTRUCT, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW, TME_LEAVE, TRACKMOUSEEVENT, VK_BACK,
    VK_DELETE, VK_ESCAPE, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONUP, WM_MOUSELEAVE,
    WM_MOUSEMOVE, WM_NCCREATE, WM_PAINT, WNDCLASSW, WS_CAPTION, WS_OVERLAPPED, WS_SYSMENU,
};

use crate::browsers::Browser;
use crate::config;
use crate::register;
use crate::ui::gdi::{
    self, ACCENT, BG, BLACK, DIM, DIM2, DT_GLYPH, DT_LINE, PANEL, PANEL2, TEXT,
};

const PAD: i32 = 16;
const WIDTH: i32 = 500;
const ROW_H: i32 = 40;
const GAP: i32 = 6;
const BSZ: i32 = 30;
const ICON: i32 = 24;
const ACTION_H: i32 = 40;
const ACTION_GAP: i32 = 6;

const ACTIONS: [&str; 3] = ["Set as default browser", "Register", "Unregister"];

#[derive(Clone, Copy, PartialEq)]
enum Region {
    Letter(usize),
    Star(usize),
    Up(usize),
    Down(usize),
    Action(usize),
}

struct RowRects {
    letter: RECT,
    star: RECT,
    up: RECT,
    down: RECT,
}

struct State {
    browsers: Vec<Browser>,
    icons: Vec<Option<HICON>>,
    favorite: Option<String>,
    configured: HashMap<String, char>,
    capturing: Option<usize>,
    status: String,
    registered: bool,
    hovered: Option<Region>,
    tracking: bool,
    font_title: HFONT,
    font_body: HFONT,
    font_dim: HFONT,
    font_glyph: HFONT,
    rows_top: i32,
    rows: Vec<RowRects>,
    actions: [RECT; 3],
}

impl State {
    fn save(&self) {
        let order: Vec<String> = self.browsers.iter().map(|b| b.id()).collect();
        config::save(&order, self.favorite.as_deref(), &self.configured);
    }
}

impl Drop for State {
    fn drop(&mut self) {
        unsafe {
            for ic in self.icons.iter().flatten() {
                DestroyIcon(*ic);
            }
            for f in [self.font_title, self.font_body, self.font_dim, self.font_glyph] {
                if !f.is_null() {
                    DeleteObject(f as HGDIOBJ);
                }
            }
        }
    }
}

pub fn run() {
    unsafe {
        let me = std::env::current_exe().ok();
        let list = crate::browsers::detect(me.as_deref());
        let settings = config::load();
        let (browsers, _) = config::apply(list, &settings);
        let icons: Vec<Option<HICON>> =
            browsers.iter().map(|b| gdi::extract_icon(&b.exe)).collect();
        let registered = register::is_registered();

        let hinstance = GetModuleHandleW(null());
        let class_name = gdi::wide("BrowserPickerSettings");
        let mut wc: WNDCLASSW = zeroed();
        wc.style = CS_HREDRAW | CS_VREDRAW;
        wc.lpfnWndProc = Some(wnd_proc);
        wc.hInstance = hinstance;
        wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
        wc.lpszClassName = class_name.as_ptr();
        RegisterClassW(&wc);

        let rows_top = PAD + 30 + 22 + 10;
        let n = browsers.len() as i32;
        let rows_block = if n > 0 { n * ROW_H + (n - 1) * GAP } else { 0 };
        let actions_top = rows_top + rows_block + if n > 0 { 16 } else { 0 };
        let action_block = 3 * ACTION_H + 2 * ACTION_GAP;
        let status_top = actions_top + action_block + 10;
        let client_h = status_top + 22 + PAD;

        let mut rows = Vec::with_capacity(browsers.len());
        for i in 0..n {
            let top = rows_top + i * (ROW_H + GAP);
            let by = top + (ROW_H - BSZ) / 2;
            let right = WIDTH - PAD;
            let down = rect(right - BSZ, by, right, by + BSZ);
            let up = rect(down.left - GAP - BSZ, by, down.left - GAP, by + BSZ);
            let star = rect(up.left - GAP - BSZ, by, up.left - GAP, by + BSZ);
            let letter = rect(star.left - GAP - BSZ, by, star.left - GAP, by + BSZ);
            rows.push(RowRects {
                letter,
                star,
                up,
                down,
            });
        }
        let mut actions = [zeroed::<RECT>(); 3];
        for (k, a) in actions.iter_mut().enumerate() {
            let top = actions_top + k as i32 * (ACTION_H + ACTION_GAP);
            *a = rect(PAD, top, WIDTH - PAD, top + ACTION_H);
        }

        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
        let mut rc = rect(0, 0, WIDTH, client_h);
        AdjustWindowRectEx(&mut rc, style, FALSE, 0);
        let win_w = rc.right - rc.left;
        let win_h = rc.bottom - rc.top;
        let x = ((GetSystemMetrics(SM_CXSCREEN) - win_w) / 2).max(0);
        let y = ((GetSystemMetrics(SM_CYSCREEN) - win_h) / 2).max(0);

        let status = if registered {
            "Registered.".to_string()
        } else {
            "Not registered yet.".to_string()
        };
        let state = Box::new(State {
            browsers,
            icons,
            favorite: settings.favorite,
            configured: settings.letters,
            capturing: None,
            status,
            registered,
            hovered: None,
            tracking: false,
            font_title: gdi::make_font(20, 600),
            font_body: gdi::make_font(15, 400),
            font_dim: gdi::make_font(13, 400),
            font_glyph: gdi::make_font(15, 400),
            rows_top,
            rows,
            actions,
        });
        let state_ptr = Box::into_raw(state);

        let title_w = gdi::wide(crate::PRODUCT_NAME);
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
            return;
        }

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

        let _ = Box::from_raw(state_ptr);
    }
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

fn in_rect(r: &RECT, x: i32, y: i32) -> bool {
    x >= r.left && x <= r.right && y >= r.top && y <= r.bottom
}

fn hit_test(state: &State, x: i32, y: i32) -> Option<Region> {
    for (i, rr) in state.rows.iter().enumerate() {
        if in_rect(&rr.letter, x, y) {
            return Some(Region::Letter(i));
        }
        if in_rect(&rr.star, x, y) {
            return Some(Region::Star(i));
        }
        if in_rect(&rr.up, x, y) {
            return Some(Region::Up(i));
        }
        if in_rect(&rr.down, x, y) {
            return Some(Region::Down(i));
        }
    }
    for (k, a) in state.actions.iter().enumerate() {
        if in_rect(a, x, y) {
            return Some(Region::Action(k));
        }
    }
    None
}

fn open_default_apps() {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", "ms-settings:defaultapps"])
        .spawn();
}

unsafe fn perform(state: &mut State, region: Region) {
    match region {
        Region::Star(i) => {
            let id = state.browsers[i].id();
            state.favorite = if state.favorite.as_deref() == Some(id.as_str()) {
                None
            } else {
                Some(id)
            };
            state.save();
        }
        Region::Up(i) => {
            if i > 0 {
                state.browsers.swap(i, i - 1);
                state.icons.swap(i, i - 1);
                state.save();
            }
        }
        Region::Down(i) => {
            if i + 1 < state.browsers.len() {
                state.browsers.swap(i, i + 1);
                state.icons.swap(i, i + 1);
                state.save();
            }
        }
        Region::Action(0) => match register::register() {
            Ok(_) => {
                open_default_apps();
                state.registered = true;
                state.status =
                    "Opened Default Apps — set Browser Picker for HTTP and HTTPS.".to_string();
            }
            Err(e) => state.status = format!("Register failed: {e}"),
        },
        Region::Action(1) => match register::register() {
            Ok(_) => {
                state.registered = true;
                state.status = "Registered.".to_string();
            }
            Err(e) => state.status = format!("Failed: {e}"),
        },
        Region::Action(_) => {
            let _ = register::unregister();
            state.registered = false;
            state.status = "Unregistered.".to_string();
        }
        // Letter capture is handled in WM_LBUTTONUP, not here.
        Region::Letter(_) => {}
    }
}

unsafe fn fill_round(hdc: winapi::shared::windef::HDC, r: &RECT, color: u32, radius: i32) {
    let brush = CreateSolidBrush(color);
    let pen = CreatePen(PS_SOLID as i32, 1, color);
    let ob = SelectObject(hdc, brush as HGDIOBJ);
    let op = SelectObject(hdc, pen as HGDIOBJ);
    RoundRect(hdc, r.left, r.top, r.right, r.bottom, radius, radius);
    SelectObject(hdc, ob);
    SelectObject(hdc, op);
    DeleteObject(brush as HGDIOBJ);
    DeleteObject(pen as HGDIOBJ);
}

unsafe fn button(
    state: &State,
    hdc: winapi::shared::windef::HDC,
    r: &RECT,
    glyph: &str,
    active: bool,
    hovered: bool,
) {
    let bg = if active || hovered { ACCENT } else { PANEL2 };
    fill_round(hdc, r, bg, 8);
    let col = if active || hovered { BLACK } else { TEXT };
    let mut gr = *r;
    gdi::draw_text(hdc, state.font_glyph, col, glyph, &mut gr, DT_GLYPH);
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

    let mut tr = rect(PAD, PAD, w - PAD, PAD + 30);
    gdi::draw_text(memdc, state.font_title, TEXT, crate::PRODUCT_NAME, &mut tr, DT_LINE);
    let mut ir = rect(PAD, PAD + 30, w - PAD, PAD + 52);
    gdi::draw_text(
        memdc,
        state.font_dim,
        DIM,
        "letter: click the box, press a key (Del clears)   ·   ★ default   ·   ▲ ▼ reorder",
        &mut ir,
        DT_LINE,
    );

    let letters = config::effective_letters(&state.browsers, &state.configured);

    for (i, b) in state.browsers.iter().enumerate() {
        let rr = &state.rows[i];
        let top = state.rows_top + i as i32 * (ROW_H + GAP);
        let row = rect(PAD, top, w - PAD, top + ROW_H);
        fill_round(memdc, &row, PANEL, 12);

        if let Some(Some(ic)) = state.icons.get(i) {
            gdi::draw_icon(memdc, row.left + 10, top + (ROW_H - ICON) / 2, ICON, *ic);
        }
        let mut nr = rect(row.left + 44, top, rr.letter.left - 10, top + ROW_H);
        gdi::draw_text(memdc, state.font_body, TEXT, &b.name, &mut nr, DT_LINE);

        // Letter box.
        let capturing = state.capturing == Some(i);
        let glyph = if capturing {
            "?".to_string()
        } else {
            letters[i].map(|c| c.to_string()).unwrap_or_else(|| "·".to_string())
        };
        button(
            state,
            memdc,
            &rr.letter,
            &glyph,
            capturing,
            state.hovered == Some(Region::Letter(i)),
        );

        let is_fav = state.favorite.as_deref() == Some(b.id().as_str());
        button(
            state,
            memdc,
            &rr.star,
            if is_fav { "★" } else { "☆" },
            is_fav,
            state.hovered == Some(Region::Star(i)),
        );
        button(state, memdc, &rr.up, "▲", false, state.hovered == Some(Region::Up(i)));
        button(state, memdc, &rr.down, "▼", false, state.hovered == Some(Region::Down(i)));
    }

    for (k, a) in state.actions.iter().enumerate() {
        let hovered = state.hovered == Some(Region::Action(k));
        fill_round(memdc, a, if hovered { ACCENT } else { PANEL }, 12);
        let col = if hovered { BLACK } else { TEXT };
        let mut lr = rect(a.left + 16, a.top, a.right - 12, a.bottom);
        gdi::draw_text(memdc, state.font_body, col, ACTIONS[k], &mut lr, DT_LINE);
    }

    let mut sr = rect(PAD, h - PAD - 20, w - PAD, h - PAD);
    gdi::draw_text(memdc, state.font_dim, DIM2, &state.status, &mut sr, DT_LINE);

    BitBlt(hdc, 0, 0, w, h, memdc, 0, 0, SRCCOPY);
    SelectObject(memdc, oldbmp);
    DeleteObject(membmp as HGDIOBJ);
    DeleteDC(memdc);
    EndPaint(hwnd, &ps);
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
            let hovered = hit_test(state, x, y);
            if hovered != state.hovered {
                state.hovered = hovered;
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
            match hit_test(state, x, y) {
                Some(Region::Letter(i)) => {
                    state.capturing = if state.capturing == Some(i) { None } else { Some(i) };
                }
                Some(other) => {
                    state.capturing = None;
                    perform(state, other);
                }
                None => state.capturing = None,
            }
            state.hovered = hit_test(state, x, y);
            InvalidateRect(hwnd, null(), FALSE);
            0
        }
        WM_KEYDOWN => {
            let vk = wparam as i32;
            if let Some(i) = state.capturing {
                if (0x41..=0x5A).contains(&vk) {
                    let c = (vk as u8) as char;
                    let id = state.browsers[i].id();
                    // Keep configured letters unique: drop this letter elsewhere.
                    state.configured.retain(|_, v| *v != c);
                    state.configured.insert(id, c);
                    state.capturing = None;
                    state.save();
                    InvalidateRect(hwnd, null(), FALSE);
                } else if vk == VK_BACK || vk == VK_DELETE {
                    let id = state.browsers[i].id();
                    state.configured.remove(&id);
                    state.capturing = None;
                    state.save();
                    InvalidateRect(hwnd, null(), FALSE);
                } else if vk == VK_ESCAPE {
                    state.capturing = None;
                    InvalidateRect(hwnd, null(), FALSE);
                }
            } else if vk == VK_ESCAPE {
                DestroyWindow(hwnd);
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
