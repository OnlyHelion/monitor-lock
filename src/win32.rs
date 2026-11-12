//! The real Win32 implementation of the SystemAccess seam. Thin layer — no
//! logic beyond calling the right API; verified by the user's in-game
//! hand-check, not by unit tests.
//!
//! Per-monitor v2 DPI awareness must be enabled before any window or tray
//! creation (`set_per_monitor_dpi_awareness`, called first in main): under
//! PMv2 every geometry here is in physical pixels, so the clip rect and the
//! picker's yellow border align exactly with monitor edges (user story 21).

use crate::seam::{MonitorInfo, Point, Rect, SystemAccess, WindowInfo};

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, ClipCursor, CreateIcon, DispatchMessageW, EnumWindows, GA_ROOT, GetAncestor, GetClassNameW,
    GetClipCursor, GetCursorPos, GetForegroundWindow, GetMessageW, GetShellWindow,
    GetSystemMetrics, GetWindowRect, HICON, ICON_BIG, ICON_SMALL, GetWindowTextW, GetWindowThreadProcessId, MSG, MSLLHOOKSTRUCT,
    SM_CXICON, SM_CXSMICON, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE,
    SW_SHOWNORMAL, SendMessageW, SetCursorPos, SetForegroundWindow, SetWindowsHookExW, ShowWindow,
    TranslateMessage, WH_MOUSE_LL, WM_MOUSEMOVE, WM_SETICON, WindowFromPoint,
};

/// Release any cursor clip. `ClipCursor(NULL)` is the Win32 way: there is no
/// "UnClipCursor" function (an earlier version looked one up by name, found
/// none, and panicked at startup; ticket #5, 2026-11-12).
fn release_clip() {
    unsafe {
        let _ = ClipCursor(None);
    }
}

/// The whole virtual screen (every monitor): what the clip reads as when none
/// is set.
fn virtual_screen() -> Rect {
    unsafe {
        Rect {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
            width: GetSystemMetrics(SM_CXVIRTUALSCREEN),
            height: GetSystemMetrics(SM_CYVIRTUALSCREEN),
        }
    }
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
    let mut handles: Vec<HMONITOR> = Vec::with_capacity(16);
    let ok = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(monitor_enum),
            // The callback reinterprets LPARAM as *mut Vec<HMONITOR> — pass
            // the Vec itself, never its buffer (ticket #5, 2026-11-12: the
            // buffer pointer made every push land in the wrong place and the
            // list come back empty, killing both auto-follow and the picker).
            LPARAM(&mut handles as *mut Vec<HMONITOR> as isize),
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

/// True when `hwnd` is the desktop or part of it: the shell window (Progman)
/// or any window rooted at it (SHELLDLL_DefView, the SysListView32 icon list
/// view, WorkerW — probed 2026-11-12, each spanning the whole virtual screen
/// 0,0,4480x1440 on the test machine). Those are shell UI, never Game
/// windows; without this filter the desktop covers every monitor 100% and
/// the Lock engages on the background — even when a desktop icon is
/// selected (ticket #5, 2026-11-12). A real app's foreground window roots
/// at its own top-level window, never at the shell.
fn is_desktop(hwnd: HWND) -> bool {
    let shell = unsafe { GetShellWindow() };
    !shell.is_invalid() && unsafe { GetAncestor(hwnd, GA_ROOT) } == shell
}

/// True for the window classes of shell UI that is not a real app window:
/// the Alt+Tab / Task View switcher, the taskbar and its tray overflow, and
/// the Start / Search flyouts. While one is foreground the seam reports "no
/// real foreground window" (like the desktop), so the Lock suspends and
/// never engages on it (user decision 2026-11-12: "alt tab unlocks it
/// immediately no matter what").
fn is_shell_ui_class(class: &str) -> bool {
    matches!(
        class,
        // Windows 11 Alt+Tab / Win+Tab host.
        "XamlExplorerHostIslandWindow"
            // Windows 10 Alt+Tab / Task View.
            | "MultitaskingViewFrame"
            | "TaskSwitcherWnd"
            | "TaskSwitcherOverlayWnd"
            // Classic Alt+Tab, plus the XAML staging window the shell focuses
            // while switching.
            | "#32771"
            | "ForegroundStaging"
            // The taskbar on the primary / secondary monitors and the tray
            // overflow flyout.
            | "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "NotifyIconOverflowWindow"
            | "TopLevelWindowForOverflowXamlIsland"
            // Start, Search and the other shell flyouts. (A UWP app's
            // foreground window is its ApplicationFrameWindow, not this.)
            | "Windows.UI.Core.CoreWindow"
    )
}

fn window_class(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

/// IN — foreground window (identity + title + bounds). None = the desktop or
/// the Alt+Tab switcher — never a Game window.
fn foreground_window() -> Option<WindowInfo> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return None;
    }
    if is_desktop(hwnd) || is_shell_ui_class(&window_class(hwnd)) {
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
/// engaged). ClipCursor REPLACES any existing clip (verified 2026-11-12: A
/// then a disjoint B reads back as B), so there is no clear-first step: a
/// clear in between would open a window in which the cursor can leave.
fn set_cursor_clip(rect: Rect) {
    guard_arm(rect);
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
    guard_disarm();
    release_clip();
}

// --- The mouse guard -------------------------------------------------------
//
// ClipCursor alone is not enough: other programs (a game, Magpie) can release
// or replace the clip every frame (user report 2026-11-12, Star Citizen +
// Magpie: the clip was changed 2138 times and the cursor leaked between
// re-applies). While the Lock is engaged, a low-level mouse hook therefore
// also clamps every mouse move that lands outside the target monitor back
// into it, whatever the clip is at that instant. The hook only reads these
// atomics; the brain's set/clear clip actions arm and disarm it, so it is
// engaged exactly when the Lock is.

static GUARD_ON: AtomicBool = AtomicBool::new(false);
static GUARD_L: AtomicI32 = AtomicI32::new(0);
static GUARD_T: AtomicI32 = AtomicI32::new(0);
static GUARD_R: AtomicI32 = AtomicI32::new(0);
static GUARD_B: AtomicI32 = AtomicI32::new(0);

fn guard_arm(rect: Rect) {
    GUARD_L.store(rect.x, Ordering::SeqCst);
    GUARD_T.store(rect.y, Ordering::SeqCst);
    GUARD_R.store(rect.x + rect.width, Ordering::SeqCst);
    GUARD_B.store(rect.y + rect.height, Ordering::SeqCst);
    GUARD_ON.store(true, Ordering::SeqCst);
}

fn guard_disarm() {
    GUARD_ON.store(false, Ordering::SeqCst);
}

/// Where a mouse move to (x, y) must land while the guard is armed with the
/// rect (l, t, r, b) (right and bottom exclusive): None when it is already
/// inside, else the nearest point inside.
fn guard_clamp(x: i32, y: i32, l: i32, t: i32, r: i32, b: i32) -> Option<(i32, i32)> {
    if r <= l || b <= t || (x >= l && x < r && y >= t && y < b) {
        return None;
    }
    Some((x.clamp(l, r - 1), y.clamp(t, b - 1)))
}

unsafe extern "system" fn mouse_guard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && wparam.0 as u32 == WM_MOUSEMOVE && GUARD_ON.load(Ordering::SeqCst) {
        let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        let (l, t, r, b) = (
            GUARD_L.load(Ordering::SeqCst),
            GUARD_T.load(Ordering::SeqCst),
            GUARD_R.load(Ordering::SeqCst),
            GUARD_B.load(Ordering::SeqCst),
        );
        if let Some((x, y)) = guard_clamp(info.pt.x, info.pt.y, l, t, r, b) {
            unsafe {
                let _ = SetCursorPos(x, y);
            }
            return LRESULT(1); // swallow the move that would have left
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Install the mouse guard's low-level hook on its own thread (a hook needs
/// a message loop). Idempotent in effect: call once at startup. The guard
/// stays disarmed until the brain engages the Lock.
pub fn start_mouse_guard() {
    std::thread::spawn(|| unsafe {
        if SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_guard_proc), None, 0).is_err() {
            return;
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

/// Release everything the Lock holds: the guard and the clip. For the panic
/// hook, so a crashed poll thread can never leave the cursor trapped.
pub fn emergency_release() {
    guard_disarm();
    release_clip();
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

/// Show and focus this process's "Monitor Lock" window, straight through
/// Win32. The tray's Open / left-click goes through here so it works while
/// the window is hidden, whether or not the hidden window is being repainted.
pub fn show_main_window() {
    set_main_window_visible(true);
}

/// Hide the window (close-to-tray). Done through Win32, not an egui viewport
/// command, so egui keeps believing the window is visible and keeps running
/// its frames: reopening never depends on a frame that a hidden egui window
/// might skip.
pub fn hide_main_window() {
    set_main_window_visible(false);
}

fn set_main_window_visible(visible: bool) {
    let Some(hwnd) = find_main_window() else {
        return;
    };
    unsafe {
        if visible {
            let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
            let _ = SetForegroundWindow(hwnd);
        } else {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

/// Set the window's icons for one Lock state: both the small one (title
/// bar) and the big one (taskbar, Alt+Tab). eframe's `ViewportCommand::Icon`
/// cannot do this: winit's `set_window_icon` sets only the small icon, so the
/// taskbar kept the startup icon (user report, v0.1.2).
pub fn set_main_window_icon(engaged: bool) {
    // One (small, big) HICON pair per state, created once and kept for the
    // process's lifetime. Stored as isize: HICON is not Send/Sync.
    static ICONS: OnceLock<[(isize, isize); 2]> = OnceLock::new();
    let icons = ICONS.get_or_init(|| {
        let pair = |engaged: bool| {
            let small = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16) as u32;
            let big = unsafe { GetSystemMetrics(SM_CXICON) }.max(32) as u32;
            (
                create_icon(small, engaged).map_or(0, |h| h.0 as isize),
                create_icon(big, engaged).map_or(0, |h| h.0 as isize),
            )
        };
        [pair(false), pair(true)]
    });
    let (small, big) = icons[usize::from(engaged)];
    let Some(hwnd) = find_main_window() else {
        return;
    };
    for (kind, icon) in [(ICON_SMALL, small), (ICON_BIG, big)] {
        if icon != 0 {
            unsafe {
                SendMessageW(hwnd, WM_SETICON, Some(WPARAM(kind as usize)), Some(LPARAM(icon)));
            }
        }
    }
}

/// An HICON of the app icon (`icon::rgba`) at `size` x `size`: 32bpp BGRA
/// color with alpha, plus an all-zero AND mask (alpha does the masking).
fn create_icon(size: u32, engaged: bool) -> Option<HICON> {
    let mut bgra = crate::icon::rgba(size, engaged);
    for px in bgra.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
    // 1bpp mask rows are WORD-aligned.
    let mask = vec![0u8; (size.div_ceil(16) * 2 * size) as usize];
    unsafe {
        CreateIcon(None, size as i32, size as i32, 1, 32, mask.as_ptr(), bgra.as_ptr()).ok()
    }
}

/// This process's "Monitor Lock" window (the egui window), if it exists.
fn find_main_window() -> Option<HWND> {
    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == std::process::id() {
            let mut buf = [0u16; 64];
            let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
            if len > 0 && String::from_utf16_lossy(&buf[..len as usize]) == "Monitor Lock" {
                unsafe { *(lparam.0 as *mut HWND) = hwnd };
            }
        }
        windows::core::BOOL(1)
    }
    let mut found = HWND::default();
    unsafe {
        let _ = EnumWindows(Some(visit), LPARAM(&mut found as *mut HWND as isize));
    }
    (!found.is_invalid()).then_some(found)
}

/// IN — a mouse button (left, right or middle) is down, or was pressed since
/// the last GetAsyncKeyState call (the low-order bit), so a click shorter than
/// one poll tick still registers. A click on one of this process's own
/// windows (tray menu, on-demand window) reports false, so using the
/// utility's own UI never counts as "a click" for the pin rules.
fn mouse_button_down() -> bool {
    let down_or_pressed = |vk: u16| unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8001 != 0 };
    // Evaluate all three (no short-circuit): each call consumes that
    // button's "pressed since last call" bit.
    let l = down_or_pressed(VK_LBUTTON.0);
    let r = down_or_pressed(VK_RBUTTON.0);
    let m = down_or_pressed(VK_MBUTTON.0);
    if !(l || r || m) {
        return false;
    }
    let mut p = POINT::default();
    if unsafe { GetCursorPos(&mut p) }.is_ok() {
        let hwnd = unsafe { WindowFromPoint(p) };
        if !hwnd.is_invalid() {
            let mut pid = 0u32;
            unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
            if pid == std::process::id() {
                return false;
            }
        }
    }
    true
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

    fn cursor_clip(&self) -> Rect {
        let mut r = RECT::default();
        if unsafe { GetClipCursor(&mut r) }.is_ok() {
            rect_from_win32(&r)
        } else {
            virtual_screen()
        }
    }

    fn mouse_button_down(&self) -> bool {
        mouse_button_down()
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

#[cfg(test)]
mod tests {
    #[test]
    fn guard_clamp_pulls_a_move_back_inside_and_leaves_inside_moves_alone() {
        // Monitor 0,0 2560x1440: right and bottom are exclusive.
        assert_eq!(super::guard_clamp(100, 100, 0, 0, 2560, 1440), None);
        assert_eq!(
            super::guard_clamp(2560, 700, 0, 0, 2560, 1440),
            Some((2559, 700))
        );
        assert_eq!(
            super::guard_clamp(3500, -40, 0, 0, 2560, 1440),
            Some((2559, 0))
        );
        // A monitor with negative coordinates.
        assert_eq!(
            super::guard_clamp(-1930, 500, -1920, 0, 0, 1080),
            Some((-1920, 500))
        );
        // A degenerate rect never traps the cursor.
        assert_eq!(super::guard_clamp(5, 5, 10, 10, 10, 10), None);
    }

    #[test]
    fn real_system_clip_reads_back_exactly_what_was_set() {
        use crate::seam::SystemAccess;
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut sys = super::Win32System;
        let monitors = sys.monitors();
        for m in &monitors {
            sys.set_cursor_clip(m.bounds);
            let got = sys.cursor_clip();
            sys.clear_cursor_clip();
            assert_eq!(got, m.bounds, "clip read-back for monitor {}", m.id);
        }
    }

    #[test]
    fn alt_tab_switcher_classes_are_filtered() {
        assert!(super::is_shell_ui_class("XamlExplorerHostIslandWindow"));
        assert!(super::is_shell_ui_class("Shell_TrayWnd"));
        assert!(super::is_shell_ui_class("Shell_SecondaryTrayWnd"));
        assert!(super::is_shell_ui_class("MultitaskingViewFrame"));
        assert!(!super::is_shell_ui_class("Chrome_WidgetWin_1"));
        assert!(!super::is_shell_ui_class("CabinetWClass"));
    }

    use crate::seam::SystemAccess;

    /// Real-seam smoke test (ticket #5 diagnosis, 2026-11-12): a running
    /// Windows desktop always has at least one monitor, so an empty result
    /// means the EnumDisplayMonitors callback path is broken — which takes
    /// out everything downstream (the fills-monitor rule, the picker's
    /// hover, and pinning). The brain is unit-tested with a fake system;
    /// this is the one guard on the real IN path.
    /// The window icon swap (taskbar + title bar) needs real HICONs; a bad
    /// buffer or mask size makes CreateIcon fail and the swap silently skips.
    #[test]
    fn real_system_creates_window_icons_for_both_states() {
        for size in [16, 32, 48] {
            for engaged in [false, true] {
                assert!(
                    super::create_icon(size, engaged).is_some(),
                    "CreateIcon failed at {size}px (engaged: {engaged})"
                );
            }
        }
    }

    #[test]
    fn real_system_reports_at_least_one_monitor() {
        let sys = super::Win32System;
        let mons = sys.monitors();
        assert!(
            !mons.is_empty(),
            "monitors() returned no monitors — EnumDisplayMonitors callback bug"
        );
    }

    /// Real-seam regression guard (ticket #5, 2026-11-12): releasing the
    /// clip must work on a real desktop and never panic (the first version
    /// panicked here and killed the poll thread at startup).
    #[test]
    fn real_system_releases_the_cursor_clip_without_panic() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        super::release_clip();
    }

    /// The real-seam tests share the one cursor and clip; run them one at a
    /// time.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// The mouse guard holds the cursor in the target monitor even when the
    /// clip is released behind the Lock's back (what Star Citizen / Magpie
    /// do). Moves the real cursor for a moment.
    #[test]
    fn real_system_guard_holds_the_cursor_when_the_clip_is_released() {
        use crate::seam::SystemAccess;
        use windows::Win32::UI::Input::KeyboardAndMouse::{MOUSEEVENTF_MOVE, mouse_event};
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        super::start_mouse_guard();
        std::thread::sleep(std::time::Duration::from_millis(300)); // hook up
        let mut sys = super::Win32System;
        // The left-most monitor, flicked to the right: with a second monitor
        // on that side the cursor would escape without the guard. A single
        // monitor makes the test vacuous, so it is skipped.
        let monitors = sys.monitors();
        let m = monitors
            .iter()
            .min_by_key(|m| m.bounds.x)
            .expect("a monitor")
            .clone();
        if !monitors
            .iter()
            .any(|o| o.bounds.x >= m.bounds.x + m.bounds.width)
        {
            return;
        }
        let before = sys.cursor_position();
        sys.set_cursor_clip(m.bounds);
        // Something else releases the clip.
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::ClipCursor(None);
        }
        let start = m.bounds.center();
        sys.move_cursor(start);
        // A hard sweep to the right, in steps (one huge step is clamped by
        // Windows itself and proves nothing), well past the monitor's edge.
        for _ in 0..80 {
            unsafe { mouse_event(MOUSEEVENTF_MOVE, 60, 0, 0, 0) };
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        let after = sys.cursor_position();
        sys.clear_cursor_clip();
        sys.move_cursor(before);
        assert!(
            m.bounds.contains(after),
            "the guard keeps the cursor inside {:?}, got {:?}",
            m.bounds,
            after
        );
    }

    /// Real-seam regression guard (ticket #5, 2026-11-12): the desktop and
    /// all of its children (SHELLDLL_DefView, the SysListView32 icon list
    /// view, WorkerW) span the whole virtual screen on the test machine and
    /// would otherwise count as Game windows covering every monitor 100% —
    /// clicking the background or a desktop icon engaged the Lock. The
    /// filter must identify the shell window and every one of its children
    /// as desktop UI.
    #[test]
    fn real_system_desktop_and_its_children_are_filtered() {
        use windows::Win32::Foundation::{HWND, LPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, GetShellWindow};
        use windows::core::BOOL;

        struct Counter {
            children: usize,
            unfiltered: usize,
        }
        unsafe extern "system" fn count_shell_child(h: HWND, lparam: LPARAM) -> BOOL {
            let c = unsafe { &mut *(lparam.0 as *mut Counter) };
            c.children += 1;
            if !super::is_desktop(h) {
                c.unfiltered += 1;
            }
            BOOL(1)
        }

        let shell = unsafe { GetShellWindow() };
        assert!(!shell.is_invalid(), "the shell window exists");
        assert!(
            super::is_desktop(shell),
            "the shell window (Progman) itself must be filtered as desktop"
        );

        let counter = Box::new(Counter {
            children: 0,
            unfiltered: 0,
        });
        let ptr = Box::into_raw(counter);
        unsafe {
            let _ = EnumChildWindows(Some(shell), Some(count_shell_child), LPARAM(ptr as isize));
        }
        let counter = unsafe { Box::from_raw(ptr) };
        assert!(
            counter.children > 0,
            "the shell window has children (the desktop UI)"
        );
        assert_eq!(
            counter.unfiltered, 0,
            "every shell child (desktop UI) must be filtered as desktop"
        );
    }
}
