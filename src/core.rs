//! LockCore: the pure brain. It takes observations from the SystemAccess seam
//! and produces decisions and actions; it makes no Windows calls and is fully
//! unit-tested against a FakeSystem.
//!
//! The fills-monitor rule: the foreground window is a Game window iff it
//! covers at least FILL_THRESHOLD of ONE monitor's FULL bounds (rcMonitor,
//! including the area under the taskbar — not the work area). The monitor it
//! fills is the Game monitor.
//!
//! The Auto-follow focus state machine: the Lock is engaged only while a Game
//! window is foreground (the cursor is clipped to the Game monitor's FULL
//! bounds); any foreground change away from it releases the Lock immediately
//! and unconditionally (that focus-loss release IS the Escape hatch — there
//! is no temporary-release hotkey); the Lock re-engages when the Game window
//! regains focus, re-centering the cursor if it sits outside the target
//! monitor. While released, no clip is set.

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

/// The brain. Owns the Lock state (the Auto-follow focus state machine) and
/// the tunable threshold; it makes no Windows calls of its own.
pub struct LockCore {
    threshold: f64,
    /// The current Lock state across ticks. Released at construction.
    lock: LockState,
}

impl LockCore {
    pub fn new() -> Self {
        Self {
            threshold: FILL_THRESHOLD,
            lock: LockState::Released,
        }
    }

    /// Clip lifecycle, startup: clear the stale cursor clip left by a
    /// previous (crashed) run before the first engagement (user story 19).
    /// The cursor clip is global Windows state that survives process death,
    /// so a crash cannot be trusted to have released it.
    pub fn startup(&mut self, sys: &mut dyn SystemAccess) {
        sys.clear_cursor_clip();
    }

    /// Clip lifecycle, exit: the clip is released when the utility exits
    /// (user story 20) — unconditionally, so it is cleared even while the
    /// Lock is engaged (the user picks Exit while the Game window is
    /// foreground).
    ///
    /// The real app never calls this: its Exit path runs on the UI thread
    /// (tray menu → `win32::clear_clip` → `process::exit`), which the poll
    /// thread's brain cannot intercept. It exists so the lifecycle contract
    /// is testable through the same seam.
    #[allow(dead_code)]
    pub fn exit(&mut self, sys: &mut dyn SystemAccess) {
        sys.clear_cursor_clip();
    }

    /// One Auto-follow focus tick: read the observation, run the state
    /// machine, and execute the resulting actions through the seam.
    pub fn update(&mut self, sys: &mut dyn SystemAccess) -> LockSnapshot {
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

        // The Auto-follow focus state machine.
        match (&game_window, &self.lock) {
            // ENGAGE: a Game window is foreground and the Lock is not engaged
            // on it — a fresh engage, a re-engage after any focus loss, or
            // focus moving directly from one Game window to another (a
            // focus loss from the first).
            (Some(game), lock) if Self::engaged_identity(lock) != Some(game.identity) => {
                if matches!(lock, LockState::Engaged { .. }) {
                    // The Lock first releases the Game window it was
                    // following, then re-engages the new one.
                    sys.clear_cursor_clip();
                }
                sys.set_cursor_clip(game.game_monitor_bounds);
                self.recenter_if_outside(sys, game.game_monitor_bounds);
                self.lock = LockState::Engaged {
                    window_identity: game.identity,
                    window_title: game.title.clone(),
                    monitor_id: game.game_monitor_id.clone(),
                    monitor_bounds: game.game_monitor_bounds,
                };
            }
            // TARGET CHANGE: the engaged Game window still has focus but now
            // fills a different monitor (it was moved or resized): the
            // target follows the Game monitor.
            (Some(game), LockState::Engaged { monitor_bounds, .. })
                if *monitor_bounds != game.game_monitor_bounds =>
            {
                sys.set_cursor_clip(game.game_monitor_bounds);
                self.recenter_if_outside(sys, game.game_monitor_bounds);
                self.lock = LockState::Engaged {
                    window_identity: game.identity,
                    window_title: game.title.clone(),
                    monitor_id: game.game_monitor_id.clone(),
                    monitor_bounds: game.game_monitor_bounds,
                };
            }
            // RELEASE: the foreground is no longer a Game window (another
            // window or the desktop) while the Lock is engaged. The release
            // is immediate and unconditional — that focus-loss release IS the
            // Escape hatch. No clip is set while released.
            (None, LockState::Engaged { .. }) => {
                sys.clear_cursor_clip();
                self.lock = LockState::Released;
            }
            // HOLD: already engaged on the foreground Game window, or
            // released with no Game window. No actions — in particular no
            // clip is set while released.
            _ => {}
        }

        LockSnapshot {
            monitors,
            foreground,
            game_window,
            lock: self.lock.clone(),
        }
    }

    /// The Game window the Lock is currently engaged on, if any.
    fn engaged_identity(lock: &LockState) -> Option<usize> {
        match lock {
            LockState::Released => None,
            LockState::Engaged {
                window_identity, ..
            } => Some(*window_identity),
        }
    }

    /// Re-center on engage: if the cursor is outside the target monitor's
    /// bounds at this moment, move it to the center of the target monitor's
    /// FULL bounds. Virtual-screen math: negative coordinates included.
    fn recenter_if_outside(&self, sys: &mut dyn SystemAccess, target: Rect) {
        let cursor = sys.cursor_position();
        if !target.contains(cursor) {
            sys.move_cursor(target.center());
        }
    }
}

impl Default for LockCore {
    fn default() -> Self {
        Self::new()
    }
}

/// The Lock state after a tick.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum LockState {
    /// Released: no clip is set; the cursor is free on all monitors.
    #[default]
    Released,
    /// Engaged: the cursor is clipped to the target monitor's FULL bounds.
    Engaged {
        /// The identity of the Game window the Lock is following.
        window_identity: usize,
        /// The Game window's title, for the on-demand window.
        window_title: String,
        /// The target monitor's id — the Game monitor in this ticket.
        monitor_id: String,
        /// The target monitor's FULL bounds (the clip rect).
        monitor_bounds: Rect,
    },
}

/// The brain's output for one tick: the raw observation plus the Lock state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LockSnapshot {
    pub monitors: Vec<MonitorInfo>,
    pub foreground: Option<WindowInfo>,
    /// Set when the foreground window is a Game window; then `game_monitor_*`
    /// name its Game monitor.
    pub game_window: Option<DetectedGameWindow>,
    /// The Lock state (engaged / released); while engaged, `Engaged` names
    /// the current target monitor.
    pub lock: LockState,
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
    use crate::fake::{Action, FakeSystem};
    use crate::seam::{MonitorInfo, Point, WindowInfo};

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

    /// Drive the brain through the seam, the way the poll thread does: one
    /// tick with a fresh brain.
    fn observe(fake: &mut FakeSystem) -> LockSnapshot {
        let mut brain = LockCore::new();
        brain.update(fake)
    }

    #[test]
    fn desktop_is_never_a_game_window() {
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![monitor("m1", 0, 0, 1920, 1080)]);
        fake.set_foreground(None); // the desktop: no foreground window

        let snap = observe(&mut fake);

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

        let snap = observe(&mut fake);

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

        let snap = observe(&mut fake);

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

        let snap = observe(&mut fake);

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

        let snap = observe(&mut fake);

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

        let snap = observe(&mut fake);

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

        let snap = observe(&mut fake);

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

        let snap = observe(&mut fake);

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

        let snap = observe(&mut fake);

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

        let snap = observe(&mut fake);

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

        let snap = observe(&mut fake);

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

        let snap = observe(&mut fake);

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

        let snap = observe(&mut fake);

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

        let snap = observe(&mut fake);

        assert_eq!(snap.monitors.len(), 2, "monitor list (count + bounds) is carried");
        assert_eq!(snap.monitors[1].bounds, Rect { x: 1920, y: 0, width: 2560, height: 1440 });
        assert_eq!(
            snap.foreground.as_ref().map(|w| (w.identity, w.title.as_str())),
            Some((7, "Game")),
            "the foreground window (identity + title) is carried"
        );
        assert!(snap.game_window.is_some());
    }

    // --- Auto-follow focus state machine (ticket #3) ---

    /// Two-monitor layout used by the state-machine tests: m1 is the Game
    /// monitor, m2 the neighbor the cursor escapes to.
    fn two_monitors(fake: &mut FakeSystem) {
        fake.set_monitors(vec![
            monitor("m1", 0, 0, 1920, 1080),
            monitor("m2", 1920, 0, 2560, 1440),
        ]);
    }

    fn game_on_m1(identity: usize) -> WindowInfo {
        window(identity, "Game", 0, 0, 1920, 1080)
    }

    const M1: Rect = Rect { x: 0, y: 0, width: 1920, height: 1080 };

    /// A browser window that fills the work area of m1 — just under the
    /// fills-monitor threshold, so never a Game window.
    fn maximized_browser() -> WindowInfo {
        window(200, "Browser", 0, 0, 1920, 1032)
    }

    #[test]
    fn engage_clips_the_game_monitor_and_leaves_the_cursor_alone() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 960, y: 540 }); // inside the Game monitor

        let snap = observe(&mut fake);

        assert!(
            matches!(snap.lock, LockState::Engaged { .. }),
            "a monitor-filling foreground window engages the Lock"
        );
        assert_eq!(
            fake.clip_sets(),
            vec![M1],
            "the cursor clip is set to the Game monitor's FULL bounds"
        );
        assert!(
            fake.cursor_moves().is_empty(),
            "the cursor is inside the target monitor: no re-centering"
        );
    }

    #[test]
    fn focus_loss_to_another_window_releases_the_lock() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 960, y: 540 });
        let mut brain = LockCore::new();
        brain.update(&mut fake); // engage

        // Alt+Tab to a browser.
        fake.set_foreground(Some(maximized_browser()));
        let snap = brain.update(&mut fake);

        assert!(
            matches!(snap.lock, LockState::Released),
            "any foreground change away from the Game window releases the Lock"
        );
        assert_eq!(fake.clip_clears(), 1, "the clip is cleared on release");
        assert_eq!(
            fake.effective_clip(),
            None,
            "release leaves no clip set — the cursor is free on all monitors"
        );
    }

    #[test]
    fn focus_loss_to_the_desktop_releases_the_lock() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 960, y: 540 });
        let mut brain = LockCore::new();
        brain.update(&mut fake); // engage

        // The window is closed / the user hits the desktop.
        fake.set_foreground(None);
        let snap = brain.update(&mut fake);

        assert!(
            matches!(snap.lock, LockState::Released),
            "losing foreground to the desktop releases the Lock"
        );
        assert_eq!(fake.clip_clears(), 1, "the clip is cleared on release");
        assert_eq!(fake.effective_clip(), None, "no clip is set while released");
    }

    #[test]
    fn released_state_never_sets_a_clip() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        let mut brain = LockCore::new();
        brain.update(&mut fake); // engage

        fake.set_foreground(Some(maximized_browser()));
        brain.update(&mut fake); // release
        fake.set_foreground(None);
        brain.update(&mut fake); // desktop
        fake.set_foreground(Some(maximized_browser()));
        brain.update(&mut fake); // browser again

        assert_eq!(
            fake.clip_sets(),
            vec![M1],
            "no clip is set while the Lock is released"
        );
        assert_eq!(
            fake.clip_clears(),
            1,
            "one release, one clear — repeated released ticks do nothing"
        );
        assert_eq!(fake.effective_clip(), None);
    }

    #[test]
    fn non_game_foreground_never_engages_the_lock() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        // A maximized-style browser fills only the work area (95.6% of the
        // full bounds) — never a Game window, even from a cold start.
        fake.set_foreground(Some(maximized_browser()));
        fake.set_cursor(Point { x: 960, y: 540 });

        let snap = observe(&mut fake);

        assert!(
            matches!(snap.lock, LockState::Released),
            "a non-Game foreground window never engages the Lock"
        );
        assert!(
            fake.actions.is_empty(),
            "no actions at all: no clip set, no clear, no cursor move"
        );
    }

    #[test]
    fn engage_release_reengage_full_cycle() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 960, y: 540 });
        let mut brain = LockCore::new();

        brain.update(&mut fake); // engage
        fake.set_foreground(Some(maximized_browser()));
        brain.update(&mut fake); // focus loss: release
        fake.set_foreground(Some(game_on_m1(100)));
        let snap = brain.update(&mut fake); // focus regained: re-engage

        assert_eq!(
            fake.actions,
            vec![Action::ClipSet(M1), Action::ClipClear, Action::ClipSet(M1)],
            "the Auto-follow focus cycle: engage, release on focus loss, re-engage"
        );
        assert!(
            matches!(
                snap.lock,
                LockState::Engaged {
                    window_identity: 100,
                    monitor_id,
                    ..
                } if monitor_id == "m1"
            ),
            "the Lock is engaged on the Game window again, targeting the Game monitor"
        );
        assert!(
            fake.cursor_moves().is_empty(),
            "the cursor is inside the target monitor: no re-centering"
        );
    }

    #[test]
    fn focus_moving_directly_to_another_game_window_releases_then_re_engages() {
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![
            monitor("m1", 0, 0, 1920, 1080),
            monitor("m2", 1920, 0, 2560, 1440),
        ]);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 960, y: 540 });
        let mut brain = LockCore::new();
        brain.update(&mut fake); // engage on m1

        // Alt+Tab straight to a second Game window on the other monitor —
        // a focus loss from the first window.
        fake.set_foreground(Some(window(300, "Other Game", 1920, 0, 2560, 1440)));
        let snap = brain.update(&mut fake);

        assert!(
            matches!(
                snap.lock,
                LockState::Engaged {
                    window_identity: 300,
                    monitor_id,
                    ..
                } if monitor_id == "m2"
            ),
            "the Lock now follows the new Game window's Game monitor"
        );
        assert_eq!(
            fake.actions,
            vec![
                Action::ClipSet(M1),
                Action::ClipClear,
                Action::ClipSet(Rect { x: 1920, y: 0, width: 2560, height: 1440 }),
                // The cursor (on m1, outside the new target) is re-centered
                // on m2.
                Action::CursorMove(Point { x: 3200, y: 720 }),
            ],
            "the old confinement is released, then the new one is set, and the stranded cursor is re-centered"
        );
    }

    #[test]
    fn engage_recenters_the_cursor_on_another_monitor() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 2500, y: 700 }); // sitting on m2

        let snap = observe(&mut fake);

        assert!(
            matches!(snap.lock, LockState::Engaged { .. }),
            "the Lock engages even though the cursor is on another monitor"
        );
        assert_eq!(
            fake.clip_sets(),
            vec![M1],
            "the clip goes to the Game monitor"
        );
        assert_eq!(
            fake.cursor_moves(),
            vec![Point { x: 960, y: 540 }],
            "the cursor is moved to the center of the Game monitor's FULL bounds"
        );
    }

    #[test]
    fn reengage_recenters_the_cursor_when_it_is_on_another_monitor() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 960, y: 540 });
        let mut brain = LockCore::new();
        brain.update(&mut fake); // engage, cursor inside

        // The Escape hatch: focus loss; the cursor wanders to m2.
        fake.set_foreground(Some(maximized_browser()));
        brain.update(&mut fake);
        fake.set_cursor(Point { x: 3000, y: 100 });

        // The Game window regains focus.
        fake.set_foreground(Some(game_on_m1(100)));
        let snap = brain.update(&mut fake);

        assert!(matches!(snap.lock, LockState::Engaged { .. }));
        assert_eq!(
            fake.cursor_moves(),
            vec![Point { x: 960, y: 540 }],
            "re-engage moves a stranded cursor back to the center of the target monitor"
        );
        assert_eq!(fake.effective_clip(), Some(M1));
    }

    #[test]
    fn recenter_uses_virtual_screen_math_with_negative_coordinates() {
        // The Game monitor is LEFT of the origin in virtual-screen space.
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![
            monitor("m1", -1920, 0, 1920, 1080),
            monitor("m2", 0, 0, 1920, 1080),
        ]);
        fake.set_foreground(Some(window(100, "Game", -1920, 0, 1920, 1080)));
        fake.set_cursor(Point { x: 500, y: 540 }); // on m2

        let snap = observe(&mut fake);

        assert!(matches!(snap.lock, LockState::Engaged { .. }));
        assert_eq!(
            fake.clip_sets(),
            vec![Rect { x: -1920, y: 0, width: 1920, height: 1080 }],
            "the clip is the Game monitor's FULL bounds (negative x)"
        );
        assert_eq!(
            fake.cursor_moves(),
            vec![Point { x: -960, y: 540 }],
            "the center of a negative-coordinate monitor is computed with virtual-screen math"
        );
    }

    #[test]
    fn cursor_exactly_on_the_right_edge_of_the_target_is_outside() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        // x == 1920 is the first pixel of m2: outside m1.
        fake.set_cursor(Point { x: 1920, y: 540 });

        observe(&mut fake);

        assert_eq!(
            fake.cursor_moves(),
            vec![Point { x: 960, y: 540 }],
            "a cursor on the target's right edge is outside and gets re-centered"
        );
    }

    #[test]
    fn startup_clears_a_stale_clip_before_any_engagement() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(None);
        let mut brain = LockCore::new();

        brain.startup(&mut fake);

        assert_eq!(
            fake.actions,
            vec![Action::ClipClear],
            "the stale clip left by a crashed previous run is cleared at startup"
        );
        // And the first tick after startup adds nothing while no Game window
        // is foreground.
        let snap = brain.update(&mut fake);
        assert!(matches!(snap.lock, LockState::Released));
        assert_eq!(
            fake.actions,
            vec![Action::ClipClear],
            "no extra actions before the first engagement"
        );
    }

    #[test]
    fn exit_clears_the_clip_while_engaged() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 960, y: 540 });
        let mut brain = LockCore::new();
        brain.update(&mut fake); // engage: clip set

        brain.exit(&mut fake);

        assert_eq!(
            fake.effective_clip(),
            None,
            "the clip is released on exit, even while the Lock is engaged"
        );
        assert_eq!(
            fake.actions.last(),
            Some(&Action::ClipClear),
            "the last action on exit is a clip clear"
        );
    }

    #[test]
    fn exit_clears_the_clip_while_released() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(None);
        let mut brain = LockCore::new();

        brain.exit(&mut fake);

        assert_eq!(
            fake.actions,
            vec![Action::ClipClear],
            "exit cleanup is unconditional — the clip cannot survive the run"
        );
    }

    #[test]
    fn target_follows_the_game_window_between_monitors() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 960, y: 540 });
        let mut brain = LockCore::new();
        brain.update(&mut fake); // engage on m1

        // The same Game window (same identity, still foreground) now fills
        // m2 — it was moved (e.g. Win+Arrow) or resized.
        fake.set_foreground(Some(window(100, "Game", 1920, 0, 2560, 1440)));
        let snap = brain.update(&mut fake);

        assert!(
            matches!(
                snap.lock,
                LockState::Engaged {
                    window_identity: 100,
                    monitor_id,
                    monitor_bounds,
                    ..
                } if monitor_id == "m2"
                    && monitor_bounds == Rect { x: 1920, y: 0, width: 2560, height: 1440 }
            ),
            "the target follows the Game window to its new Game monitor"
        );
        assert_eq!(
            fake.clip_sets(),
            vec![M1, Rect { x: 1920, y: 0, width: 2560, height: 1440 }],
            "the clip is re-set to the new Game monitor"
        );
        assert_eq!(
            fake.clip_clears(),
            0,
            "no focus change happened: this is not a release"
        );
        assert_eq!(
            fake.cursor_moves(),
            vec![Point { x: 3200, y: 720 }],
            "the cursor, now outside the new target, is re-centered — otherwise it would freeze outside the clip"
        );
    }
}
