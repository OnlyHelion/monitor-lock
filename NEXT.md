# NEXT — Monitor Lock
Updated: 2026-11-12 15:05 · Claude Code (Opus 5.5) · 3c1d589 + ledger/NEXT.md uncommitted (master)

## Next step
User's hand-check of v0.1.2 at runtime: tray + taskbar show the dark-gray monitor while released, amber + padlock while engaged.

## State now
- v0.1.2 is the latest GitHub release, published 2026-11-12 14:49 with monitor-lock.exe + .sha256 (Release run 43229657834 success; gh release view). Commit 3c1d589 on master.
- v0.1.2 = new monitor icon (src/icon.rs `icon::rgba(size, engaged)`): released = dark gray screen, engaged = amber screen + padlock; no power dot. Used for the tray, the window icon (replaces eframe's default egui logo; swaps with the tray) and res/icon.ico (engaged art, 6 sizes).
- cargo test 68/68, clippy clean, local release exe's extracted icon is the new monitor without the dot.
- Pre-check OK, awaiting user: tray + taskbar icon swap while the app runs.
- UNVERIFIED: pin-mode Alt+Tab release on this Win11 build; if Alt+Tab still holds the lock, read the foreground class during Alt+Tab and add it to `is_shell_ui_class` (src/win32.rs).

## Don't retry
- Hovering the Monitor Lock window changes the clip (suspected winit); the workaround is closing to the tray. Details in the ledger.
- In .rc files `#` is a preprocessor char (RC1021), not a comment — use /* */.

## Open questions
- User asked about releases building on their own: tag pushes already do; auto-tagging on a Cargo.toml version bump was offered, not built.

## Pointers
- docs/ledgers/monitor-lock.md (top section: v0.1.2 monitor icon)
- Regenerate res/icon.ico after changing src/icon.rs: `MONITOR_LOCK_WRITE_ICO=1 cargo test ico_file`
- Release: .github/workflows/release.yml (push a v* tag)
