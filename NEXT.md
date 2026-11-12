# NEXT — Monitor Lock
Updated: 2026-11-12 15:35 · Claude Code (Opus 5.5) · handoff-docs commit on top of f58312b, pushed (master)

## Next step
User's hand-check of v0.1.3: the taskbar icon turns amber + padlock when the Lock engages, dark gray when released.

## State now
- v0.1.3 is the latest GitHub release, published 2026-11-12 15:09 with monitor-lock.exe + .sha256 (Release run 43233242369 success; gh release view). Commit f58312b on master.
- v0.1.3 fix: win32::set_main_window_icon sets BOTH ICON_SMALL and ICON_BIG via WM_SETICON on each state change (winit's set_window_icon sets only the small one, so the taskbar never changed). cargo test 69/69, clippy clean.
- Pre-check OK, awaiting user: taskbar icon swap at runtime.
- The v0.1.2 exe's icons are correct (downloaded + ExtractIconEx); the padlock the user saw in Explorer was Windows' icon cache for the old file at the same path (`ie4uinit.exe -show` or rename).
- UNVERIFIED: pin-mode Alt+Tab release on this Win11 build; if Alt+Tab still holds the lock, read the foreground class during Alt+Tab and add it to `is_shell_ui_class` (src/win32.rs).

## Don't retry
- egui ViewportCommand::Icon for the taskbar: winit sets only ICON_SMALL; use win32::set_main_window_icon.
- Hovering the Monitor Lock window changes the clip (suspected winit); the workaround is closing to the tray. Details in the ledger.
- In .rc files `#` is a preprocessor char (RC1021), not a comment — use /* */.

## Open questions
- Auto-release when Cargo.toml's version changes: offered, not built.

## Pointers
- docs/ledgers/monitor-lock.md (top sections: v0.1.3 taskbar icon swap, v0.1.2 monitor icon)
- Regenerate res/icon.ico after changing src/icon.rs: `MONITOR_LOCK_WRITE_ICO=1 cargo test ico_file`
- Release: .github/workflows/release.yml (push a v* tag)
