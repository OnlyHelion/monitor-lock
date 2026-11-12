//! LockCore: the pure brain. It takes observations from the SystemAccess seam
//! and produces decisions; it makes no Windows calls and is fully unit-tested
//! against a FakeSystem.
//!
//! The one rule in this ticket is the fills-monitor rule: the foreground
//! window is a Game window iff it covers at least FILL_THRESHOLD of ONE
//! monitor's FULL bounds (rcMonitor, including the area under the taskbar —
//! not the work area). The monitor it fills is the Game monitor.

use crate::seam::{MonitorInfo, Rect, SystemAccess, WindowInfo};

/// Fills-monitor threshold: the fraction of a monitor's FULL bounds the
/// foreground window must cover to be a Game window.
///
/// Set above the user's measured work-area fraction (95.6–96.7% of full
/// bounds) so a maximized browser — sized to the work area — does NOT count
/// as a Game window. The spec's "~95%" is superseded by that settled edge;
/// this is the single tunable constant, and the in-game hand-check (ticket #5)
/// is the arbiter if a "borderless maximized" game covering only the work
/// area must engage instead.
pub const FILL_THRESHOLD: f64 = 0.97;

/// The brain. Holds the tunable threshold; the fills-monitor rule is a pure
/// function of one observation.
#[derive(Clone, Copy)]
pub struct LockCore {
    threshold: f64,
}

impl LockCore {
    pub fn new() -> Self {
        Self {
            threshold: FILL_THRESHOLD,
        }
    }

    /// Read the current observation from the seam and decide whether the
    /// foreground window is a Game window.
    pub fn observe(&self, sys: &dyn SystemAccess) -> LockSnapshot {
        let monitors = sys.monitors();
        let foreground = sys.foreground_window();
        let game_window = foreground.as_ref().and_then(|window| {
            filled_monitor(&monitors, window, self.threshold).map(|(id, bounds)| DetectedGameWindow {
                identity: window.identity,
                title: window.title.clone(),
                game_monitor_id: id,
                game_monitor_bounds: bounds,
            })
        });
        LockSnapshot {
            monitors,
            foreground,
            game_window,
        }
    }
}

impl Default for LockCore {
    fn default() -> Self {
        Self::new()
    }
}

/// The brain's decision for one observation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LockSnapshot {
    pub monitors: Vec<MonitorInfo>,
    pub foreground: Option<WindowInfo>,
    /// Set when the foreground window is a Game window; then `game_monitor_*`
    /// name its Game monitor.
    pub game_window: Option<DetectedGameWindow>,
}

/// A detected Game window and its Game monitor.
#[derive(Clone, Debug, PartialEq)]
pub struct DetectedGameWindow {
    pub identity: usize,
    pub title: String,
    pub game_monitor_id: String,
    pub game_monitor_bounds: Rect,
}

/// The fills-monitor rule: does `window` cover at least `threshold` of ONE
/// monitor's FULL bounds? Returns the Game monitor's (id, bounds), or None.
///
/// A window slightly larger than its monitor still fills it (the intersection
/// is the whole monitor). A window straddling two monitors fills whichever of
/// them it covers best. The desktop (no foreground window) never gets here —
/// callers pass a real window only.
pub fn filled_monitor(
    monitors: &[MonitorInfo],
    window: &WindowInfo,
    threshold: f64,
) -> Option<(String, Rect)> {
    let mut best: Option<(f64, String, Rect)> = None;
    for monitor in monitors {
        let monitor_area = monitor.bounds.area();
        if monitor_area == 0 {
            continue;
        }
        let covered = window
            .bounds
            .intersection(&monitor.bounds)
            .map(|r| r.area() as f64)
            .unwrap_or(0.0);
        let fraction = covered / monitor_area as f64;
        if fraction >= threshold
            && best.as_ref().map(|(f, _, _)| fraction > *f).unwrap_or(true)
        {
            best = Some((
                fraction,
                monitor.id.clone(),
                monitor.bounds,
            ));
        }
    }
    best.map(|(_, id, bounds)| (id, bounds))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeSystem;
    use crate::seam::{MonitorInfo, WindowInfo};

    fn monitor(id: &str, x: i32, y: i32, w: i32, h: i32) -> MonitorInfo {
        MonitorInfo {
            id: id.into(),
            bounds: Rect {
                x,
                y,
                width: w,
                height: h,
            },
        }
    }

    fn window(identity: usize, title: &str, x: i32, y: i32, w: i32, h: i32) -> WindowInfo {
        WindowInfo {
            identity,
            title: title.into(),
            bounds: Rect {
                x,
                y,
                width: w,
                height: h,
            },
        }
    }

    /// Drive the brain through the seam, the way the poll thread does.
    fn observe(fake: &FakeSystem) -> LockSnapshot {
        LockCore::new().observe(fake)
    }

    #[test]
    fn desktop_is_never_a_game_window() {
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![monitor("m1", 0, 0, 1920, 1080)]);
        fake.set_foreground(None); // the desktop: no foreground window

        let snap = observe(&fake);

        assert!(
            snap.game_window.is_none(),
            "the desktop (no foreground window) is never a Game window"
        );
    }

    #[test]
    fn work_area_sized_window_does_not_fill_full_bounds() {
        // 1080p monitor, 48 px taskbar: the work area is 1920x1032 = 95.6% of
        // the full bounds. A maximized browser (work-area sized) must NOT
        // count as a Game window.
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![monitor("m1", 0, 0, 1920, 1080)]);
        fake.set_foreground(Some(window(1, "Browser", 0, 0, 1920, 1032)));

        let snap = observe(&fake);

        assert!(
            snap.game_window.is_none(),
            "a work-area-sized (maximized-style) window does not fill the full bounds"
        );
    }

    #[test]
    fn work_area_sized_window_1440p_also_does_not_fill() {
        // 1440p monitor, 48 px taskbar: work area 2560x1392 = 96.7% of the
        // full bounds — the top of the user's measured range. Still below
        // FILL_THRESHOLD.
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![monitor("m1", 0, 0, 2560, 1440)]);
        fake.set_foreground(Some(window(1, "Browser", 0, 0, 2560, 1392)));

        let snap = observe(&fake);

        assert!(
            snap.game_window.is_none(),
            "a 1440p work-area-sized window (96.7% of full bounds) does not fill"
        );
    }

    #[test]
    fn window_slightly_larger_than_its_monitor_fills_it() {
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![monitor("m1", 0, 0, 1000, 1000)]);
        // 50 px bleed past every edge: the intersection is the whole monitor.
        fake.set_foreground(Some(window(1, "Game", -50, -50, 1100, 1100)));

        let snap = observe(&fake);

        assert_eq!(
            snap.game_window.as_ref().map(|g| g.game_monitor_id.as_str()),
            Some("m1"),
            "a window slightly larger than its monitor still fills it"
        );
    }

    #[test]
    fn full_bounds_window_is_a_game_window() {
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![monitor("m1", 0, 0, 1920, 1080)]);
        fake.set_foreground(Some(window(1, "Game", 0, 0, 1920, 1080)));

        let snap = observe(&fake);

        assert_eq!(
            snap.game_window.as_ref().map(|g| g.game_monitor_id.as_str()),
            Some("m1"),
            "a window covering the monitor's full bounds is a Game window"
        );
        assert_eq!(
            snap.game_window.as_ref().map(|g| g.game_monitor_bounds),
            Some(Rect { x: 0, y: 0, width: 1920, height: 1080 }),
            "the Game monitor is the one the window fills"
        );
    }

    #[test]
    fn just_under_threshold_is_not_a_game_window() {
        // 1000x1000 monitor; window covers 96.9% — just under FILL_THRESHOLD.
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![monitor("m1", 0, 0, 1000, 1000)]);
        fake.set_foreground(Some(window(1, "Window", 0, 0, 1000, 969)));

        let snap = observe(&fake);

        assert!(
            snap.game_window.is_none(),
            "96.9% of the full bounds is just under the threshold"
        );
    }

    #[test]
    fn just_over_threshold_is_a_game_window() {
        // 1000x1000 monitor; window covers 97.1% — just over FILL_THRESHOLD.
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![monitor("m1", 0, 0, 1000, 1000)]);
        fake.set_foreground(Some(window(1, "Game", 0, 0, 1000, 971)));

        let snap = observe(&fake);

        assert_eq!(
            snap.game_window.as_ref().map(|g| g.game_monitor_id.as_str()),
            Some("m1"),
            "97.1% of the full bounds is just over the threshold"
        );
    }

    #[test]
    fn exactly_at_threshold_is_a_game_window() {
        // The rule is "at least" the threshold: 97.0% exactly counts.
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![monitor("m1", 0, 0, 1000, 1000)]);
        fake.set_foreground(Some(window(1, "Game", 0, 0, 1000, 970)));

        let snap = observe(&fake);

        assert_eq!(
            snap.game_window.as_ref().map(|g| g.game_monitor_id.as_str()),
            Some("m1"),
            "exactly at the threshold counts (at-least semantics)"
        );
    }

    #[test]
    fn window_straddling_two_monitors_fills_neither() {
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![
            monitor("m1", 0, 0, 1000, 1000),
            monitor("m2", 1000, 0, 1000, 1000),
        ]);
        // Exactly half of each monitor: 50% < threshold on both.
        fake.set_foreground(Some(window(1, "Window", 500, 0, 1000, 1000)));

        let snap = observe(&fake);

        assert!(
            snap.game_window.is_none(),
            "a window straddling two monitors (50% of each) fills neither"
        );
    }

    #[test]
    fn straddling_window_that_fills_one_monitor_targets_that_monitor() {
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![
            monitor("m1", 0, 0, 1000, 1000),
            monitor("m2", 1000, 0, 1000, 1000),
        ]);
        // 100% of m1, 90% of m2: fills m1 only.
        fake.set_foreground(Some(window(1, "Game", 0, 0, 1900, 1000)));

        let snap = observe(&fake);

        assert_eq!(
            snap.game_window.as_ref().map(|g| g.game_monitor_id.as_str()),
            Some("m1"),
            "the Game monitor is the one the window fills best"
        );
        assert_eq!(
            snap.game_window.as_ref().map(|g| g.game_monitor_bounds),
            Some(Rect { x: 0, y: 0, width: 1000, height: 1000 })
        );
    }

    #[test]
    fn window_on_left_negative_monitor_targets_that_monitor() {
        // Left monitor sits at negative x in virtual-screen space.
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![
            monitor("m1", -1920, 0, 1920, 1080),
            monitor("m2", 0, 0, 1920, 1080),
        ]);
        fake.set_foreground(Some(window(1, "Game", -1920, 0, 1920, 1080)));

        let snap = observe(&fake);

        assert_eq!(
            snap.game_window.as_ref().map(|g| g.game_monitor_id.as_str()),
            Some("m1"),
            "negative-coordinate geometry: the window fills the left monitor"
        );
        assert_eq!(
            snap.game_window.as_ref().map(|g| g.game_monitor_bounds),
            Some(Rect { x: -1920, y: 0, width: 1920, height: 1080 })
        );
    }

    #[test]
    fn window_on_right_monitor_targets_that_monitor() {
        // Same layout; the window is on the positive-coordinate monitor.
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![
            monitor("m1", -1920, 0, 1920, 1080),
            monitor("m2", 0, 0, 1920, 1080),
        ]);
        fake.set_foreground(Some(window(1, "Game", 0, 0, 1920, 1080)));

        let snap = observe(&fake);

        assert_eq!(
            snap.game_window.as_ref().map(|g| g.game_monitor_id.as_str()),
            Some("m2"),
            "picks the monitor the window fills, not the first one enumerated"
        );
    }

    #[test]
    fn window_outside_all_monitors_is_not_a_game_window() {
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![monitor("m1", 0, 0, 1000, 1000)]);
        fake.set_foreground(Some(window(1, "Window", 2000, 2000, 500, 500)));

        let snap = observe(&fake);

        assert!(
            snap.game_window.is_none(),
            "a window that overlaps no monitor fills no monitor"
        );
    }

    #[test]
    fn snapshot_carries_the_raw_observation_for_the_ui() {
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![
            monitor("m1", 0, 0, 1920, 1080),
            monitor("m2", 1920, 0, 2560, 1440),
        ]);
        fake.set_foreground(Some(window(7, "Game", 0, 0, 1920, 1080)));

        let snap = observe(&fake);

        assert_eq!(snap.monitors.len(), 2, "monitor list (count + bounds) is carried");
        assert_eq!(snap.monitors[1].bounds, Rect { x: 1920, y: 0, width: 2560, height: 1440 });
        assert_eq!(
            snap.foreground.as_ref().map(|w| (w.identity, w.title.as_str())),
            Some((7, "Game")),
            "the foreground window (identity + title) is carried"
        );
        assert!(snap.game_window.is_some());
    }
}
