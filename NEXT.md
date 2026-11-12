# NEXT.md — Monitor Lock

## State now
- Project: K:\monitor-lock (git; public github.com/OnlyHelion/monitor-lock). Branch `implement/spec-1-monitor-lock`, pushed after the next commit.
- #5 hand-check FAILED twice (user 2026-11-12 09:43/09:45, then panic log ~09:5x). TWO independent bugs, both fixed + verified (real-seam tests red→green, `cargo test` 45/45):
  1. `monitors()` passed the Vec's BUFFER to the `EnumDisplayMonitors` callback → monitor list always empty (commit bf3cacf).
  2. `UnClipCursor` `GetProcAddress` resolution PANICKED — the export is absent by name from this machine's user32.dll → poll thread died at startup (first stale-clip cleanup). Fix: `release_clip()` falls back to ClipCursor(whole virtual screen) when the export is missing. Detail: `docs/ledgers/monitor-lock.md` (DIAGNOSIS 09:50 + DIAGNOSIS-2 10:10 lines).
- NEW exe: `K:\monitor-lock\target\release\monitor-lock.exe` (7,964,160 bytes, 2026-11-12 10:10:16). Old process fully dead (build succeeded without lock).

## Next step
1. User re-runs the #5 hand-check on the NEW exe (steps in the #5 comment, issue comment 6812768234): borderless game filling a monitor → cursor confined to FULL bounds (incl. under taskbar); alt-tab → release; maximized browser must NOT engage; picker (tray → Pick monitor; click = pin, Esc = cancel; Back to auto); exit clears the clip.
2. On user confirmation: `gh issue close 5 --repo OnlyHelion/monitor-lock --comment '<verdict>'` (+ #1 if spec accepted). If a borderless game covering only the work area does NOT engage: lower `FILL_THRESHOLD` in `src/core.rs` (0.97; a maximized browser measures 95.6–96.7%), rebuild, retest, record the value in a #5 comment.
3. Optional clarification for the user: what "copy pasting it also crashed the app" meant (likely: copied the exe, ran the copy).

## Watch items
- This machine's user32.dll lacks `UnClipCursor` by name (unusual; ClipCursor/SetCursorPos present) — the virtual-screen fallback is what makes the app work here; keep it.
- A click on the on-demand window during the picker also counts as the global pin click; border fixed at 5 physical px; pinning does NOT make windowed games engage (open spec item, flagged to user).
- No PC shutdown while the user is testing (the 01:22 permission is superseded).
- UNTESTED-VISUAL (still): tray icon rendering, window display, overlay ring, picker feel, real clip — user validating on the new exe.
- If the user is testing again, the exe is locked: ask them to Exit before any `cargo build --release`.
- The tree is NOT `cargo fmt`-clean (app.rs/core.rs/main.rs/overlay.rs/seam.rs reformat under the current rustfmt; left as-is — don't drag into fix commits).
