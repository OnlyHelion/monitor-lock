//! FakeSystem: the test double for the SystemAccess seam. Invents monitors,
//! foreground windows and cursor positions, and records the brain's actions
//! in order so tests can assert on them. Used only by unit tests (cfg(test)).

#[cfg(test)]
use crate::seam::{MonitorInfo, Point, Rect, SystemAccess, WindowInfo};

/// One action the brain performed through the seam, in order.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// `set_cursor_clip(rect)` — the cursor clip is set to this rect.
    ClipSet(Rect),
    /// `clear_cursor_clip()` — the cursor clip is removed.
    ClipClear,
    /// `move_cursor(point)` — the cursor is moved to this point.
    CursorMove(Point),
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeSystem {
    monitors: Vec<MonitorInfo>,
    foreground: Option<WindowInfo>,
    cursor: Point,
    /// The brain's actions through the seam, in the order performed.
    pub actions: Vec<Action>,
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

    /// The set-clip actions, in order.
    pub fn clip_sets(&self) -> Vec<Rect> {
        self.actions
            .iter()
            .filter_map(|a| match a {
                Action::ClipSet(r) => Some(*r),
                _ => None,
            })
            .collect()
    }

    /// The number of clear-clip actions.
    pub fn clip_clears(&self) -> usize {
        self.actions
            .iter()
            .filter(|a| matches!(a, Action::ClipClear))
            .count()
    }

    /// The move-cursor actions, in order.
    pub fn cursor_moves(&self) -> Vec<Point> {
        self.actions
            .iter()
            .filter_map(|a| match a {
                Action::CursorMove(p) => Some(*p),
                _ => None,
            })
            .collect()
    }

    /// The clip the system would have after the recorded actions: None when
    /// the last clip action was a clear (or there was none), else the last
    /// set rect.
    pub fn effective_clip(&self) -> Option<Rect> {
        for action in self.actions.iter().rev() {
            match action {
                Action::ClipSet(rect) => return Some(*rect),
                Action::ClipClear => return None,
                Action::CursorMove(_) => {}
            }
        }
        None
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
        self.actions.push(Action::ClipSet(rect));
    }

    fn clear_cursor_clip(&mut self) {
        self.actions.push(Action::ClipClear);
    }

    fn move_cursor(&mut self, point: Point) {
        self.actions.push(Action::CursorMove(point));
    }
}
