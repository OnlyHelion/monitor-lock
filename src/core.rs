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
//!
//! The Monitor override: pinning a monitor (via the Monitor picker) changes
//! the Lock's target from the Game monitor to the pinned monitor; the
//! activation rule (a Game window is foreground) is UNCHANGED. Back to auto
//! restores following the Game monitor. No persistence: a pin lasts only for
//! the current run.
//!
//! The Monitor picker: explicit trigger only (a start-picker command from the
//! UI). While active: the clip is cleared (cursor free on all monitors), the
//! Lock is not engaged, and the brain tracks which monitor is under the
//! cursor (the hover) so the thin layer can outline it. A click (fed to the
//! brain as an observed input) pins the monitor under the cursor and ends the
//! picker; Esc (same) ends the picker without pinning. After the picker ends,
//! normal auto-follow resumes: if a Game window is still foreground, the Lock
//! re-engages on the (possibly pinned) target, including the re-center rule.

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
    /// The Monitor override: the id of the pinned monitor, if any. Resolved
    /// against the current tick's monitors (the pin is void when the pinned
    /// monitor no longer exists). No persistence: set only by the picker.
    pin: Option<String>,
    /// The Monitor picker is active: the cursor is free, the monitor under
    /// the cursor is tracked for the outline, and the state machine is
    /// suspended. Set only by an explicit start-picker command.
    picking: bool,
}

impl LockCore {
    pub fn new() -> Self {
        Self {
            threshold: FILL_THRESHOLD,
            lock: LockState::Released,
            pin: None,
            picking: false,
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

    /// Monitor picker: explicit start (a command from the UI — the window's
    /// button or the tray menu). The picker never starts on its own. While
    /// active, the clip is cleared (cursor free on all monitors) and the Lock
    /// is not engaged. A repeated start is a no-op.
    pub fn start_picker(&mut self, sys: &mut dyn SystemAccess) {
        if self.picking {
            return;
        }
        self.picking = true;
        if matches!(self.lock, LockState::Engaged { .. }) {
            sys.clear_cursor_clip();
            self.lock = LockState::Released;
        }
    }

    /// Monitor picker: the observed click. Pin the monitor under the cursor
    /// and end the picker. If the cursor is over no monitor, end without
    /// pinning. A no-op when the picker is not active.
    pub fn confirm_picker(&mut self, sys: &mut dyn SystemAccess) {
        if !self.picking {
            return;
        }
        let cursor = sys.cursor_position();
        self.pin = sys
            .monitors()
            .iter()
            .find(|m| m.bounds.contains(cursor))
            .map(|m| m.id.clone());
        self.picking = false;
    }

    /// Monitor picker: the observed Esc. End the picker without pinning —
    /// the current target (including any existing pin) is unchanged.
    pub fn cancel_picker(&mut self) {
        self.picking = false;
    }

    /// Back to auto: clear the Monitor override and resume following the
    /// Game monitor. Also ends an active picker without pinning.
    pub fn back_to_auto(&mut self) {
        self.pin = None;
        self.picking = false;
    }

    /// One Auto-follow focus tick: read the observation, run the state
    /// machine, and execute the resulting actions through the seam.
    ///
    /// While the Monitor picker is active, the state machine is suspended:
    /// the clip stays cleared (cursor free on all monitors) and the brain
    /// tracks which monitor is under the cursor (the hover) so the thin layer
    /// can outline it.
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

        // The Monitor override, resolved against this tick's monitors: the
        // pin is void when the pinned monitor no longer exists (the target
        // falls back to the Game monitor).
        let pinned = self.resolve_pin(&monitors).map(|(id, bounds)| MonitorOverride {
            monitor_id: id,
            monitor_bounds: bounds,
        });

        if self.picking {
            // The Monitor picker is active: the cursor is free on all
            // monitors (no clip), the Lock is not engaged, and the state
            // machine is suspended. Track the monitor under the cursor so
            // the thin layer knows which monitor to outline.
            let cursor = sys.cursor_position();
            let picker_hover = monitors
                .iter()
                .find(|m| m.bounds.contains(cursor))
                .cloned();
            if matches!(self.lock, LockState::Engaged { .. }) {
                // Defensive: start_picker already released the Lock.
                sys.clear_cursor_clip();
                self.lock = LockState::Released;
            }
            return LockSnapshot {
                monitors,
                foreground,
                game_window,
                lock: self.lock.clone(),
                pinned,
                picking: true,
                picker_hover,
            };
        }

        // The Lock's target: the pinned monitor when a Monitor override is
        // active, else the Game monitor.
        let target = pinned
            .as_ref()
            .map(|p| (p.monitor_id.clone(), p.monitor_bounds))
            .or_else(|| {
                game_window
                    .as_ref()
                    .map(|g| (g.game_monitor_id.clone(), g.game_monitor_bounds))
            });

        // The Auto-follow focus state machine.
        match (&game_window, &self.lock) {
            // ENGAGE: a Game window is foreground and the Lock is not engaged
            // on it — a fresh engage, a re-engage after any focus loss (or
            // the end of the Monitor picker), or focus moving directly from
            // one Game window to another (a focus loss from the first).
            (Some(game), lock) if Self::engaged_identity(lock) != Some(game.identity) => {
                let (target_id, target_bounds) = target
                    .clone()
                    .unwrap_or_else(|| (game.game_monitor_id.clone(), game.game_monitor_bounds));
                if matches!(lock, LockState::Engaged { .. }) {
                    // The Lock first releases the Game window it was
                    // following, then re-engages the new one.
                    sys.clear_cursor_clip();
                }
                sys.set_cursor_clip(target_bounds);
                self.recenter_if_outside(sys, target_bounds);
                self.lock = LockState::Engaged {
                    window_identity: game.identity,
                    window_title: game.title.clone(),
                    monitor_id: target_id,
                    monitor_bounds: target_bounds,
                };
            }
            // TARGET CHANGE: the engaged Game window still has focus but the
            // target changed — the Game window now fills a different monitor
            // (it was moved or resized), a Monitor override was set or
            // cleared, or the pinned monitor's bounds changed: the clip
            // follows the target.
            (Some(game), LockState::Engaged { monitor_bounds, .. })
                if target
                    .as_ref()
                    .map(|(_, t)| *monitor_bounds != *t)
                    .unwrap_or(false) =>
            {
                let (target_id, target_bounds) =
                    target.clone().expect("the target is set while a Game window is foreground");
                sys.set_cursor_clip(target_bounds);
                self.recenter_if_outside(sys, target_bounds);
                self.lock = LockState::Engaged {
                    window_identity: game.identity,
                    window_title: game.title.clone(),
                    monitor_id: target_id,
                    monitor_bounds: target_bounds,
                };
            }
            // RELEASE: the foreground is no longer a Game window (another
            // window or the desktop) while the Lock is engaged. The release
            // is immediate and unconditional — that focus-loss release IS the
            // Escape hatch. No clip is set while released. A Monitor
            // override is unaffected: the pin survives a focus loss.
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
            pinned,
            picking: false,
            picker_hover: None,
        }
    }

    /// The Monitor override, resolved against the current tick's monitors:
    /// the pinned monitor's current (id, bounds), or None when there is no
    /// pin — clearing it when the pinned monitor no longer exists.
    fn resolve_pin(&mut self, monitors: &[MonitorInfo]) -> Option<(String, Rect)> {
        let id = self.pin.as_deref()?;
        match monitors.iter().find(|m| m.id == id) {
            Some(m) => Some((m.id.clone(), m.bounds)),
            None => {
                self.pin = None; // the pinned monitor is gone: the override is void
                None
            }
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
        /// The target monitor's id — the Game monitor, or the pinned monitor
        /// when a Monitor override is active.
        monitor_id: String,
        /// The target monitor's FULL bounds (the clip rect).
        monitor_bounds: Rect,
    },
}

/// The brain's output for one tick: the raw observation plus the Lock state,
/// the Monitor override, and the Monitor picker state.
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
    /// The active Monitor override (a pinned monitor), if any — set even
    /// while the Lock is released (the pin outlives a focus loss).
    pub pinned: Option<MonitorOverride>,
    /// The Monitor picker is active: the cursor is free on all monitors, the
    /// monitor under the cursor is outlined, a click pins it, Esc cancels.
    pub picking: bool,
    /// While the Monitor picker is active: the monitor under the cursor (the
    /// one a click would pin), or None when the cursor is over no monitor.
    pub picker_hover: Option<MonitorInfo>,
}

/// The active Monitor override: a pinned monitor and its current FULL bounds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorOverride {
    pub monitor_id: String,
    pub monitor_bounds: Rect,
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

    // --- Monitor picker + Monitor override (ticket #4) ---

    /// The picker tests' m2 (the monitor the user pins), full bounds.
    const M2: Rect = Rect { x: 1920, y: 0, width: 2560, height: 1440 };

    #[test]
    fn start_picker_frees_the_cursor_and_releases_the_lock() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 960, y: 540 });
        let mut brain = LockCore::new();
        brain.update(&mut fake); // engage on m1
        assert_eq!(fake.effective_clip(), Some(M1));

        brain.start_picker(&mut fake);
        let snap = brain.update(&mut fake);

        assert!(snap.picking, "the picker is active after the explicit start");
        assert_eq!(
            fake.effective_clip(),
            None,
            "the cursor is free on all monitors while the picker is active"
        );
        assert!(
            matches!(snap.lock, LockState::Released),
            "the Lock is not engaged while the picker is active"
        );
        assert_eq!(
            fake.clip_sets(),
            vec![M1],
            "the state machine is suspended: no re-engage while the picker is active"
        );
    }

    #[test]
    fn start_picker_is_idempotent_and_never_starts_automatically() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 960, y: 540 });
        let mut brain = LockCore::new();

        // Ordinary poll ticks alone never start the picker, even while a
        // Game window is foreground.
        for _ in 0..3 {
            let snap = brain.update(&mut fake);
            assert!(!snap.picking, "the picker starts only explicitly, never accidentally");
        }

        brain.start_picker(&mut fake);
        brain.start_picker(&mut fake); // a repeated explicit start: no-op
        let snap = brain.update(&mut fake);
        assert!(snap.picking);
        assert_eq!(
            fake.clip_clears(),
            1,
            "a repeated start performs no extra action"
        );
    }

    #[test]
    fn picker_hover_tracks_the_cursor_across_monitors() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 960, y: 540 });
        let mut brain = LockCore::new();
        brain.start_picker(&mut fake);

        let snap = brain.update(&mut fake);
        assert_eq!(
            snap.picker_hover.as_ref().map(|m| m.id.as_str()),
            Some("m1"),
            "the monitor under the cursor is tracked for the outline"
        );

        fake.set_cursor(Point { x: 3000, y: 720 }); // on m2
        let snap = brain.update(&mut fake);
        assert_eq!(
            snap.picker_hover.as_ref().map(|m| m.id.as_str()),
            Some("m2"),
            "the hover follows the cursor across monitors"
        );
    }

    #[test]
    fn picker_hover_handles_negative_monitor_coordinates() {
        // Two monitors with a GAP between them, in negative coordinates:
        // left is -3000..-440, right is 0..1920, the gap is -440..0.
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![
            monitor("left", -3000, 0, 2560, 1440),
            monitor("right", 0, 0, 1920, 1080),
        ]);
        fake.set_foreground(None);
        let mut brain = LockCore::new();
        brain.start_picker(&mut fake);

        fake.set_cursor(Point { x: -1000, y: 540 });
        let snap = brain.update(&mut fake);
        assert_eq!(
            snap.picker_hover.as_ref().map(|m| m.id.as_str()),
            Some("left"),
            "negative virtual-screen coordinates are handled"
        );

        fake.set_cursor(Point { x: 900, y: 540 });
        let snap = brain.update(&mut fake);
        assert_eq!(
            snap.picker_hover.as_ref().map(|m| m.id.as_str()),
            Some("right"),
            "the hover moves across to the positive-coordinate monitor"
        );

        fake.set_cursor(Point { x: -200, y: 540 }); // in the gap
        let snap = brain.update(&mut fake);
        assert!(
            snap.picker_hover.is_none(),
            "a cursor outside all monitors (the gap) outlines nothing"
        );
    }

    #[test]
    fn click_pins_the_monitor_under_the_cursor() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 960, y: 540 });
        let mut brain = LockCore::new();
        brain.start_picker(&mut fake);
        brain.update(&mut fake); // hover m1

        fake.set_cursor(Point { x: 3000, y: 720 }); // move to m2
        brain.confirm_picker(&mut fake); // the observed click: pin under the cursor

        let snap = brain.update(&mut fake); // auto-follow resumes
        assert!(!snap.picking, "the click ends the picker");
        assert_eq!(
            snap.pinned.as_ref().map(|p| p.monitor_id.as_str()),
            Some("m2"),
            "the Monitor override is the monitor under the cursor at the click"
        );
        assert!(
            matches!(
                snap.lock,
                LockState::Engaged { monitor_id, .. } if monitor_id == "m2"
            ),
            "while the Game window is foreground, the Lock targets the pinned monitor"
        );
        assert_eq!(
            fake.clip_sets(),
            vec![M2],
            "the clip is set to the pinned monitor, not the Game monitor"
        );
    }

    #[test]
    fn click_with_cursor_outside_all_monitors_ends_without_pin() {
        let mut fake = FakeSystem::new();
        fake.set_monitors(vec![
            monitor("left", -3000, 0, 2560, 1440),
            monitor("right", 0, 0, 1920, 1080),
        ]);
        fake.set_foreground(None);
        let mut brain = LockCore::new();
        brain.start_picker(&mut fake);

        fake.set_cursor(Point { x: -200, y: 540 }); // in the gap
        brain.confirm_picker(&mut fake);

        let snap = brain.update(&mut fake);
        assert!(!snap.picking, "the click still ends the picker");
        assert!(
            snap.pinned.is_none(),
            "no monitor under the cursor: nothing is pinned"
        );
    }

    #[test]
    fn reengage_with_pin_recenters_the_cursor_on_the_pinned_monitor() {
        // The cursor is always ON the pinned monitor at the click (the pin is
        // resolved from the cursor), so the re-center rule for the pinned
        // target bites on the later re-engage: focus loss, the user works on
        // another monitor, the Game regains focus.
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 3000, y: 720 }); // on m2
        let mut brain = LockCore::new();
        brain.start_picker(&mut fake);
        brain.update(&mut fake);
        brain.confirm_picker(&mut fake); // pin m2
        let snap = brain.update(&mut fake);
        assert!(
            matches!(
                snap.lock,
                LockState::Engaged { monitor_id, .. } if monitor_id == "m2"
            ),
            "the Lock re-engages on the pinned monitor (cursor on m2: no re-center)"
        );
        assert!(fake.cursor_moves().is_empty());

        // Focus loss (Alt+Tab): the user works on m1, the cursor moves there.
        fake.set_foreground(None);
        fake.set_cursor(Point { x: 960, y: 540 });
        let snap = brain.update(&mut fake);
        assert!(
            matches!(snap.lock, LockState::Released),
            "the focus loss releases the Lock (the Escape hatch)"
        );

        // The Game regains focus: re-engage on the pinned monitor, and the
        // cursor (on m1) is re-centered on m2.
        fake.set_foreground(Some(game_on_m1(100)));
        let snap = brain.update(&mut fake);
        assert!(
            matches!(
                snap.lock,
                LockState::Engaged { monitor_id, .. } if monitor_id == "m2"
            ),
            "the re-engage targets the pinned monitor"
        );
        assert_eq!(
            fake.cursor_moves(),
            vec![Point { x: 3200, y: 720 }],
            "the re-center rule applies to the pinned target: the cursor, outside m2, is moved to m2's center"
        );
    }

    #[test]
    fn esc_cancels_the_picker_without_changing_the_target() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 3000, y: 720 });
        let mut brain = LockCore::new();
        // First pin m2...
        brain.start_picker(&mut fake);
        brain.update(&mut fake);
        brain.confirm_picker(&mut fake);
        let snap = brain.update(&mut fake);
        assert!(
            matches!(
                snap.lock,
                LockState::Engaged { monitor_id, .. } if monitor_id == "m2"
            ),
            "setup: the Lock is engaged on the pinned m2"
        );

        // ...then start the picker again, hover m1, and Esc: the pin survives.
        brain.start_picker(&mut fake);
        fake.set_cursor(Point { x: 960, y: 540 }); // hover m1
        brain.cancel_picker(); // Esc

        let snap = brain.update(&mut fake);
        assert!(!snap.picking, "Esc ends the picker");
        assert_eq!(
            snap.pinned.as_ref().map(|p| p.monitor_id.as_str()),
            Some("m2"),
            "Esc does not pin: the current target is unchanged"
        );
        assert!(
            matches!(
                snap.lock,
                LockState::Engaged { monitor_id, .. } if monitor_id == "m2"
            ),
            "auto-follow resumes on the unchanged target"
        );
    }

    #[test]
    fn esc_with_no_existing_pin_keeps_the_game_monitor_target() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 3000, y: 720 }); // hover m2
        let mut brain = LockCore::new();
        brain.start_picker(&mut fake);
        brain.update(&mut fake);
        brain.cancel_picker(); // Esc

        let snap = brain.update(&mut fake);
        assert!(!snap.picking, "Esc ends the picker");
        assert!(snap.pinned.is_none(), "Esc pins nothing");
        assert!(
            matches!(
                snap.lock,
                LockState::Engaged { monitor_id, .. } if monitor_id == "m1"
            ),
            "auto-follow resumes on the Game monitor"
        );
        assert_eq!(fake.clip_sets(), vec![M1], "the clip targets the Game monitor");
    }

    #[test]
    fn back_to_auto_restores_following_the_game_monitor() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 3000, y: 720 });
        let mut brain = LockCore::new();
        brain.start_picker(&mut fake);
        brain.update(&mut fake);
        brain.confirm_picker(&mut fake); // pin m2
        brain.update(&mut fake); // engage on m2
        assert_eq!(fake.clip_sets(), vec![M2]);

        brain.back_to_auto();

        let snap = brain.update(&mut fake);
        assert!(snap.pinned.is_none(), "the Monitor override is cleared");
        assert!(
            matches!(
                snap.lock,
                LockState::Engaged { monitor_id, monitor_bounds, .. }
                    if monitor_id == "m1" && monitor_bounds == M1
            ),
            "the Lock targets the Game monitor again"
        );
        assert_eq!(
            fake.clip_sets(),
            vec![M2, M1],
            "the clip is re-set to the Game monitor"
        );
        assert_eq!(
            fake.cursor_moves(),
            vec![Point { x: 960, y: 540 }],
            "the cursor (on m2) is re-centered on the Game monitor"
        );
    }

    #[test]
    fn back_to_auto_also_cancels_an_active_picker() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 3000, y: 720 });
        let mut brain = LockCore::new();
        brain.start_picker(&mut fake);
        brain.update(&mut fake);

        brain.back_to_auto();

        let snap = brain.update(&mut fake);
        assert!(!snap.picking, "Back to auto ends the picker without pinning");
        assert!(snap.pinned.is_none());
        assert!(
            matches!(
                snap.lock,
                LockState::Engaged { monitor_id, .. } if monitor_id == "m1"
            ),
            "following the Game monitor resumes"
        );
    }

    #[test]
    fn a_pinned_monitor_that_disappears_falls_back_to_the_game_monitor() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 3000, y: 720 });
        let mut brain = LockCore::new();
        brain.start_picker(&mut fake);
        brain.update(&mut fake);
        brain.confirm_picker(&mut fake); // pin m2
        brain.update(&mut fake); // engage on m2

        // m2 is unplugged: only m1 remains.
        fake.set_monitors(vec![monitor("m1", 0, 0, 1920, 1080)]);
        let snap = brain.update(&mut fake);

        assert!(
            snap.pinned.is_none(),
            "the override is void when the pinned monitor no longer exists"
        );
        assert!(
            matches!(
                snap.lock,
                LockState::Engaged { monitor_id, monitor_bounds, .. }
                    if monitor_id == "m1" && monitor_bounds == M1
            ),
            "the Lock follows the Game monitor"
        );
    }

    #[test]
    fn a_pinned_monitor_whose_bounds_change_re_targets_the_lock() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 3000, y: 720 });
        let mut brain = LockCore::new();
        brain.start_picker(&mut fake);
        brain.update(&mut fake);
        brain.confirm_picker(&mut fake); // pin m2 (2560x1440)
        brain.update(&mut fake); // engage on m2

        // m2's bounds change (resolution change): the pin follows by id, and
        // the cursor (at the old m2 center) is now outside the new bounds.
        let m2_new = Rect { x: 1920, y: 0, width: 1280, height: 720 };
        fake.set_monitors(vec![
            monitor("m1", 0, 0, 1920, 1080),
            monitor("m2", 1920, 0, 1280, 720),
        ]);
        let snap = brain.update(&mut fake);

        assert!(
            matches!(
                snap.lock,
                LockState::Engaged { monitor_id, monitor_bounds, .. }
                    if monitor_id == "m2" && monitor_bounds == m2_new
            ),
            "the pinned target follows the monitor's new bounds"
        );
        assert_eq!(
            fake.clip_sets(),
            vec![M2, m2_new],
            "the clip is re-set to the new bounds"
        );
        assert_eq!(
            fake.cursor_moves(),
            vec![Point { x: 2560, y: 360 }],
            "the cursor is outside the new bounds: re-centered"
        );
    }

    #[test]
    fn pin_does_not_change_the_activation_rule() {
        let mut fake = FakeSystem::new();
        two_monitors(&mut fake);
        fake.set_foreground(Some(game_on_m1(100)));
        fake.set_cursor(Point { x: 3000, y: 720 });
        let mut brain = LockCore::new();
        brain.start_picker(&mut fake);
        brain.update(&mut fake);
        brain.confirm_picker(&mut fake); // pin m2
        brain.update(&mut fake); // engage on m2

        // A non-Game foreground window (browser-sized) while pinned: no
        // engagement — the activation rule is unchanged by the override.
        fake.set_foreground(Some(maximized_browser()));
        let snap = brain.update(&mut fake);
        assert!(
            matches!(snap.lock, LockState::Released),
            "a non-Game foreground window never engages, even while a monitor is pinned"
        );
        assert_eq!(fake.effective_clip(), None, "no clip while released");
        assert_eq!(
            snap.pinned.as_ref().map(|p| p.monitor_id.as_str()),
            Some("m2"),
            "the pin survives a focus loss"
        );

        // The Game regains focus: the Lock re-engages on the pinned monitor.
        fake.set_foreground(Some(game_on_m1(100)));
        let snap = brain.update(&mut fake);
        assert!(
            matches!(
                snap.lock,
                LockState::Engaged { monitor_id, .. } if monitor_id == "m2"
            ),
            "re-engagement targets the pinned monitor"
        );
    }
}
