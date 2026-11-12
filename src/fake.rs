//! FakeSystem: the test double for the SystemAccess seam. Invents monitors,
//! foreground windows and cursor positions, and records the brain's actions so
//! tests can assert on them. Used only by unit tests (cfg(test)).
//!
//! The action-recording fields and cursor setter are part of the agreed seam
//! surface; ticket #3's tests assert on them, hence the allow until then.
#![allow(dead_code)]

#[cfg(test)]
use crate::seam::{MonitorInfo, Point, Rect, SystemAccess, WindowInfo};

#[cfg(test)]
#[derive(Default)]
pub struct FakeSystem {
    monitors: Vec<MonitorInfo>,
    foreground: Option<WindowInfo>,
    cursor: Point,
    /// Recorded set-clip actions, in order.
    pub clips_set: Vec<Rect>,
    /// Number of recorded clear-clip actions.
    pub clip_clears: usize,
    /// Recorded move-cursor actions, in order.
    pub cursor_moves: Vec<Point>,
}

#[cfg(test)]
impl FakeSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_monitors(&mut self, monitors: Vec<MonitorInfo>) {
        self.monitors = monitors;
    }

    /// None = the desktop (no foreground window).
    pub fn set_foreground(&mut self, foreground: Option<WindowInfo>) {
        self.foreground = foreground;
    }

    pub fn set_cursor(&mut self, cursor: Point) {
        self.cursor = cursor;
    }
}

#[cfg(test)]
impl SystemAccess for FakeSystem {
    fn monitors(&self) -> Vec<MonitorInfo> {
        self.monitors.clone()
    }

    fn foreground_window(&self) -> Option<WindowInfo> {
        self.foreground.clone()
    }

    fn cursor_position(&self) -> Point {
        self.cursor
    }

    fn set_cursor_clip(&mut self, rect: Rect) {
        self.clips_set.push(rect);
    }

    fn clear_cursor_clip(&mut self) {
        self.clip_clears += 1;
    }

    fn move_cursor(&mut self, point: Point) {
        self.cursor_moves.push(point);
    }
}
