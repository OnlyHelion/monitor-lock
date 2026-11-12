//! The real Win32 implementation of the SystemAccess seam. Thin layer — no
//! logic beyond calling the right API; verified by the user's in-game
//! hand-check, not by unit tests.
//!
//! Per-monitor v2 DPI awareness must be enabled before any window or tray
//! creation (`set_per_monitor_dpi_awareness`, called first in main): under
//! PMv2 every geometry here is in physical pixels, so the clip rect and the
//! picker's yellow border align exactly with monitor edges (user story 21).

use crate::seam::{MonitorInfo, Point, Rect, SystemAccess, WindowInfo};

use windows::core::{PCSTR, PCWSTR};
use windows::Win32::Foundation::{LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{
    ClipCursor, GetCursorPos, GetForegroundWindow, GetWindowRect, GetWindowTextW, SetCursorPos,
};

// UnClipCursor (WINBOOL WINAPI UnClipCursor(void)) is a plain user32
// function that the windows 0.62.2 bindings do not expose. It cannot be
// linked normally either: the Windows SDK 10.0.26100 user32.lib import
// library omits it (verified: ClipCursor is present, UnClipCursor is not),
// so it is resolved at runtime from the already-loaded user32.dll.
static UN_CLIP_CURSOR: std::sync::OnceLock<unsafe extern "system" fn()> =
    std::sync::OnceLock::new();

fn un_clip_cursor() {
    let f = *UN_CLIP_CURSOR.get_or_init(|| {
        unsafe {
            // user32.dll is always loaded in a Windows process.
            let module: Vec<u16> =
                "user32.dll".encode_utf16().chain(std::iter::once(0)).collect();
            let h = GetModuleHandleW(PCWSTR(module.as_ptr()))
                .expect("user32.dll is loaded in every Windows process");
            let name: Vec<u8> = b"UnClipCursor\0".to_vec();
            let Some(resolved) = GetProcAddress(h, PCSTR(name.as_ptr().cast())) else {
                panic!("UnClipCursor is a public user32 export");
            };
            // The crate types FARPROC with a placeholder return type, so
            // transmute to the real signature: WINBOOL WINAPI UnClipCursor(void).
            std::mem::transmute::<
                unsafe extern "system" fn() -> isize,
                unsafe extern "system" fn(),
            >(resolved)
        }
    });
    unsafe { f() };
}

/// Enable per-monitor v2 DPI awareness. MUST be the first statement of main,
/// before any window or tray creation. An Err return just means the process
/// already has an equal/higher awareness (e.g. set first by a manifest),
/// which is fine.
pub fn set_per_monitor_dpi_awareness() {
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}

/// The real seam. Stateless: every call goes straight to Win32.
#[derive(Clone, Copy, Default)]
pub struct Win32System;

/// IN — monitors (id + full bounds). Re-enumerated every poll tick: cheap,
/// and it picks up monitor changes (add/remove/move) without extra plumbing.
fn monitors() -> Vec<MonitorInfo> {
    // Capacity headroom so the Vec never reallocates while the enum callback
    // is pushing (the callback dereferences the original pointer each time).
    // `mut` is only used inside the callback, so the lint can't see it.
    #[allow(unused_mut)]
    let mut handles: Vec<HMONITOR> = Vec::with_capacity(16);
    let ok = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(monitor_enum),
            LPARAM(handles.as_ptr() as isize),
        )
    };
    if ok.0 == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(handles.len());
    for h in &handles {
        let mut mi = MONITORINFOEXW::default();
        // cbSize must be the size of the EX struct even though the API takes
        // the inner MONITORINFO (that is how Windows tells them apart).
        mi.monitorInfo.cbSize = core::mem::size_of::<MONITORINFOEXW>() as u32;
        let ok = unsafe { GetMonitorInfoW(*h, &mut mi.monitorInfo) };
        if ok.0 != 0 {
            // rcMonitor = FULL bounds including the area under the taskbar —
            // what the fills-monitor rule compares against (not rcWork).
            out.push(MonitorInfo {
                id: wide_string(&mi.szDevice),
                bounds: rect_from_win32(&mi.monitorInfo.rcMonitor),
            });
        }
    }
    out
}

/// EnumDisplayMonitors callback: collect the HMONITORs via LPARAM.
unsafe extern "system" fn monitor_enum(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprcmonitor: *mut RECT,
    ldata: LPARAM,
) -> windows::core::BOOL {
    unsafe {
        let handles = &mut *(ldata.0 as *mut Vec<HMONITOR>);
        handles.push(hmonitor);
    }
    windows::core::BOOL(1)
}

/// IN — foreground window (identity + title + bounds). None = the desktop —
/// never a Game window.
fn foreground_window() -> Option<WindowInfo> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return None;
    }
    let mut title_buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut title_buf) };
    // Some games report a zero-length title; the UI shows a fallback.
    let title = if len > 0 {
        String::from_utf16_lossy(&title_buf[..len as usize])
    } else {
        String::new()
    };
    let mut rect = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut rect);
    }
    Some(WindowInfo {
        identity: hwnd.0 as usize,
        title,
        bounds: rect_from_win32(&rect),
    })
}

/// IN — cursor position.
fn cursor_position() -> Point {
    let mut p = POINT::default();
    if unsafe { GetCursorPos(&mut p) }.is_ok() {
        Point { x: p.x, y: p.y }
    } else {
        Point::default()
    }
}

/// OUT — set the cursor clip to exactly `rect` (the Lock's confinement while
/// engaged). Repeated ClipCursor calls intersect, and the clip is global
/// Windows state, so any existing clip is removed first; the result is that
/// the clip is exactly `rect`.
fn set_cursor_clip(rect: Rect) {
    un_clip_cursor();
    unsafe {
        let r = RECT {
            left: rect.x,
            top: rect.y,
            right: rect.x + rect.width,
            bottom: rect.y + rect.height,
        };
        let _ = ClipCursor(Some(&r));
    }
}

/// OUT — clear the cursor clip. Called at startup (stale-clip cleanup), on
/// every Lock release, and on exit.
fn clear_cursor_clip() {
    un_clip_cursor();
}

/// OUT — move the cursor (re-center on engage).
fn move_cursor(point: Point) {
    unsafe {
        let _ = SetCursorPos(point.x, point.y);
    }
}

/// Clear any cursor clip from outside the poll loop — the tray's Exit path.
/// The clip is global Windows state that survives process death (user story
/// 20), so it must be cleared before the process goes.
pub fn clear_clip() {
    clear_cursor_clip();
}

/// IN — the Monitor picker's global Esc observation: true while the physical
/// Esc key is down. GetAsyncKeyState reports the hardware-level state and is
/// correct from the poll thread (GetKeyState is tied to the calling thread's
/// message queue — the wrong API for a background poller).
pub fn esc_down() -> bool {
    key_down(VK_ESCAPE.0)
}

/// IN — the Monitor picker's global left-click observation: true while the
/// physical left mouse button is down. The click is resolved from the cursor
/// position (the brain), not through the input-transparent overlay.
pub fn left_click_down() -> bool {
    key_down(VK_LBUTTON.0)
}

/// The high-order bit (0x8000) of GetAsyncKeyState is set while the key is
/// down.
fn key_down(vk: u16) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 }
}

fn rect_from_win32(r: &RECT) -> Rect {
    Rect {
        x: r.left,
        y: r.top,
        width: r.right - r.left,
        height: r.bottom - r.top,
    }
}

/// A null-terminated wide buffer (e.g. MONITORINFOEXW::szDevice) as a String.
fn wide_string(buf: &[u16; 32]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

impl SystemAccess for Win32System {
    fn monitors(&self) -> Vec<MonitorInfo> {
        monitors()
    }

    fn foreground_window(&self) -> Option<WindowInfo> {
        foreground_window()
    }

    fn cursor_position(&self) -> Point {
        cursor_position()
    }

    fn set_cursor_clip(&mut self, rect: Rect) {
        set_cursor_clip(rect)
    }

    fn clear_cursor_clip(&mut self) {
        clear_cursor_clip()
    }

    fn move_cursor(&mut self, point: Point) {
        move_cursor(point)
    }
}
