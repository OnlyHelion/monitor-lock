# NEXT.md — Monitor Lock

## State now
- Project: K:\monitor-lock (git; public github.com/OnlyHelion/monitor-lock). Branch `implement/spec-1-monitor-lock`.
- #5 hand-check FAILED on the 10:10 exe (user 09:43/09:45: nothing activates; ~09:5x: UnClipCursor panic in log; 10:1x: over-engagement on browser + desktop background). THREE root causes, ALL fixed + verified (real-seam tests red→green, `cargo test` 47/47):
  1. `monitors()` passed the Vec's BUFFER to the `EnumDisplayMonitors` callback → monitor list always empty (commit bf3cacf).
  2. `UnClipCursor` absent by name from this machine's user32.dll → startup panic killed the poll thread; `release_clip()` now falls back to ClipCursor(whole virtual screen) (commit dfa7e32).
  3. Over-engagement (DIAGNOSIS-3): desktop = Progman + all its children (SHELLDLL_DefView, SysListView32, WorkerW) span the whole virtual screen → `is_desktop()` filter (GetAncestor(GA_ROOT) == shell) in win32.rs; a real maximized Edge measures 97.22% of M1 (40 px taskbar + 8 px overscan) → FILL_THRESHOLD 0.97→0.98 in core.rs. Detail: `docs/ledgers/monitor-lock.md` (DIAGNOSIS-3 line).
- NEW exe: `K:\monitor-lock\target\release\monitor-lock.exe` (7,962,624 bytes, 2026-11-12 10:42:50). Fix-3 commit + push landed with this NEXT.md.

## Next step
1. User re-runs the #5 hand-check on the NEW exe (steps in the #5 comment, issue comment 6812768234): borderless game filling a monitor → cursor confined to FULL bounds (incl. under taskbar); alt-tab → release; maximized browser must NOT engage; desktop/background must NOT engage; picker (tray → Pick monitor; click = pin, Esc = cancel; Back to auto); exit clears the clip.
2. On user confirmation: `gh issue close 5 --repo OnlyHelion/monitor-lock --comment '<verdict>'` (+ #1 if spec accepted). If a borderless game covering only the work area does NOT engage: it is geometrically indistinguishable from a maximized browser (both 97.22% here) — lower `FILL_THRESHOLD` in `src/core.rs` (now 0.98) only on user request, rebuild, retest, record the value in a #5 comment.
3. Open user items: (a) pin semantics ANSWERED (NO — a pin alone never engages the Lock; spec: the activation rule is unchanged, the pin only redirects the target while a Game window has focus); (b) "copy pasting it also crashed the app" clarification pending — likely: copied the exe and ran the copy (same binary, same crash).

## Watch items
- This machine's user32.dll lacks `UnClipCursor` by name (unusual; ClipCursor/SetCursorPos present) — the virtual-screen fallback is what makes the app work here; keep it.
- A click on the on-demand window during the picker also counts as the global pin click; border fixed at 5 physical px; pinning does NOT make windowed games engage (open spec item, flagged to user).
- No PC shutdown while the user is testing (the 01:22 permission is superseded).
- UNTESTED-VISUAL (still): tray icon rendering, window display, overlay ring, picker feel, real clip — user validating on the new exe.
- If the user is testing, the exe is locked: ask them to Exit before any `cargo build --release`.
- The tree is NOT `cargo fmt`-clean (app.rs/core.rs/main.rs/overlay.rs/seam.rs reformat under the current rustfmt; left as-is — don't drag into fix commits).
