# NEXT.md — Monitor Lock

## State now
- Project: K:\monitor-lock (git; public github.com/OnlyHelion/monitor-lock). Branch `implement/spec-1-monitor-lock`.
- #5 hand-check FAILED on the 10:10 exe (user 09:43/09:45: nothing activates; ~09:5x: UnClipCursor panic in log; 10:1x: over-engagement on browser + desktop background). THREE root causes, ALL fixed + verified (real-seam tests red→green, `cargo test` 49/49):
  1. `monitors()` passed the Vec's BUFFER to the `EnumDisplayMonitors` callback → monitor list always empty (commit bf3cacf).
  2. `UnClipCursor` absent by name from this machine's user32.dll → startup panic killed the poll thread; `release_clip()` now falls back to ClipCursor(whole virtual screen) (commit dfa7e32).
  3. Over-engagement (DIAGNOSIS-3): desktop = Progman + all its children span the whole virtual screen → `is_desktop()` filter (GetAncestor(GA_ROOT) == shell) in win32.rs; a real maximized Edge measures 97.22% of M1 → FILL_THRESHOLD 0.97→0.98 in core.rs (commit 248f80d).
- NEW SPEC CHANGE — PIN-FORCE (user 11:14, "pin as backup"): while a monitor is pinned, the Lock engages on it for ANY real (non-desktop) foreground window — no fills-monitor requirement (a windowed game with a title bar now locks); the desktop background releases (Escape hatch in pin mode); focus moving between windows re-engages on the pinned monitor; Back to auto unchanged. Issue #1 body + comment updated (comment 6821195434). 3 new pin-mode tests replace the old-rule test. Detail: `docs/ledgers/monitor-lock.md` (PIN-FORCE line).
- NEW exe: `K:\monitor-lock\target\release\monitor-lock.exe` (7,964,672 bytes, 2026-11-12 11:25:48 — fixes 1–3 + pin-force). Committed + pushed with this NEXT.md.

## Next step
1. User re-runs the #5 hand-check on the NEW exe: (a) desktop/background must NOT engage; (b) maximized browser must NOT engage (auto mode); (c) borderless game filling a monitor → cursor confined to FULL bounds (incl. under taskbar), Alt+Tab → release; (d) PIN-FORCE: pick a monitor (tray → Pick monitor; click = pin, Esc = cancel) → then focus ANY window that does not fill the screen (e.g. the browser) → Lock engages on the pinned monitor; click the desktop background → release; focus a window again → re-engages; Back to auto → auto mode resumes.
2. On user confirmation: `gh issue close 5 --repo OnlyHelion/monitor-lock --comment '<verdict>'` (+ #1 if spec accepted). If a borderless game covering only the work area does NOT engage in AUTO: pin the monitor (pin-force covers it) or lower `FILL_THRESHOLD` in `src/core.rs` (now 0.98) only on user request, rebuild, retest, record the value in a #5 comment.
3. Open user items: (a) pin semantics ANSWERED, then CHANGED per user request (pin-force implemented 11:14); (b) "copy pasting it also crashed the app" clarification pending — likely: copied the exe and ran the copy (same binary, same crash).

## Watch items
- This machine's user32.dll lacks `UnClipCursor` by name (unusual; ClipCursor/SetCursorPos present) — the virtual-screen fallback is what makes the app work here; keep it.
- A click on the on-demand window during the picker also counts as the global pin click; border fixed at 5 physical px.
- No PC shutdown while the user is testing (the 01:22 permission is superseded).
- UNTESTED-VISUAL (still): tray icon rendering, window display (now includes the pin-mode hint line), overlay ring, picker feel, real clip, pin-force in real use — user validating on the new exe.
- If the user is testing, the exe is locked: ask them to Exit before any `cargo build --release`.
- The tree is NOT `cargo fmt`-clean (app.rs/core.rs/main.rs/overlay.rs/seam.rs reformat under the current rustfmt; left as-is — don't drag into fix commits).
