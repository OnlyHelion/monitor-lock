//! The SystemAccess seam: the single Windows-facing interface the brain
//! (LockCore) uses. The thin layer implements it with real Win32 calls
//! (see win32.rs); unit tests implement it with a FakeSystem (see fake.rs).
//! The brain makes no Windows calls of its own.
//!
//! All coordinates are in Windows virtual-screen space (physical pixels under
//! per-monitor v2 DPI awareness) and may be negative for monitors left of or
//! above the origin.

/// A point in virtual-screen space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// A rectangle in virtual-screen space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// Area in square pixels; 0 for degenerate rects.
    pub fn area(&self) -> i64 {
        if self.width <= 0 || self.height <= 0 {
            0
        } else {
            self.width as i64 * self.height as i64
        }
    }

    /// Overlap of two rects, or None when they do not overlap.
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);
        if x2 > x1 && y2 > y1 {
            Some(Rect {
                x: x1,
                y: y1,
                width: x2 - x1,
                height: y2 - y1,
            })
        } else {
            None
        }
    }
}

/// One monitor, as the brain sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorInfo {
    /// Stable monitor id within a run (e.g. `\\.\DISPLAY1`).
    pub id: String,
    /// FULL monitor bounds (rcMonitor) — includes the area under the taskbar,
    /// NOT the work area. The fills-monitor rule compares against these.
    pub bounds: Rect,
}

/// The foreground window, as the brain sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowInfo {
    /// Window identity (HWND as usize); the brain compares it for same-window checks.
    pub identity: usize,
    /// Window title; may be empty (some games report no title).
    pub title: String,
    /// Window bounds in virtual-screen space.
    pub bounds: Rect,
}

/// The single seam between the brain and Windows.
///
/// In: monitors, foreground window, cursor position.
/// Out: set-clip, clear-clip, move-cursor. In ticket #2 the three out actions
/// are inert stubs — no cursor locking yet (ticket #3).
pub trait SystemAccess {
    fn monitors(&self) -> Vec<MonitorInfo>;
    /// None = the desktop (no foreground window) — never a Game window.
    fn foreground_window(&self) -> Option<WindowInfo>;
    // The cursor and the three out actions are part of the agreed seam
    // surface; the brain starts using them in ticket #3 (cursor locking).
    #[allow(dead_code)]
    fn cursor_position(&self) -> Point;
    #[allow(dead_code)]
    fn set_cursor_clip(&mut self, rect: Rect);
    #[allow(dead_code)]
    fn clear_cursor_clip(&mut self);
    #[allow(dead_code)]
    fn move_cursor(&mut self, point: Point);
}
