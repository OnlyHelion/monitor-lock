//! The real Win32 implementation of the SystemAccess seam. Thin layer — no
//! logic beyond calling the right API; verified by the user's in-game
//! hand-check, not by unit tests.
//!
//! Per-monitor v2 DPI awareness must be enabled before any window or tray
//! creation (`set_per_monitor_dpi_awareness`, called first in main): under
//! PMv2 every geometry here is in physical pixels, so the clip rect and the
//! picker's yellow border align exactly with monitor edges (user story 21).

use crate::seam::{MonitorInfo, Point, Rect, SystemAccess, WindowInfo};

use windows::Win32::Foundation::{LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, GetWindowRect, GetWindowTextW,
};

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

    // Inert stubs: no cursor locking in this ticket. The real ClipCursor /
    // UnClipCursor / SetCursorPos land in ticket #3 together with the brain's
    // Lock state machine (and the startup/exit clip cleanup).
    fn set_cursor_clip(&mut self, _rect: Rect) {}

    fn clear_cursor_clip(&mut self) {}

    fn move_cursor(&mut self, _point: Point) {}
}
