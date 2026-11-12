# NEXT — Monitor Lock
Updated: 2026-11-12 14:05 · Claude Code · ab664ea clean (master)

## Next step
Nothing open. Next release: push a new tag `vX.Y.Z` on master (the `Release` workflow builds and publishes monitor-lock.exe).

## State now
- master at ab664ea, working tree clean (git status).
- v0.1.0 is the latest GitHub release, published 2026-11-12 (gh release list).
- No open issues; #7 "Follow-ups after v0.1.0" is closed (gh issue list / view).
- Cleaned 2026-11-12: only branch is master (local + origin); merged branches deleted; `cargo clean` run (no `target/` until the next build); empty `.worktrees/` removed (git branch -a, ls).
- UNVERIFIED: pin-mode Alt+Tab release on this Win11 build; if Alt+Tab still holds the lock, read the foreground class during Alt+Tab and add it to `is_shell_ui_class` (src/win32.rs).

## Don't retry
- Hovering the Monitor Lock window changes the clip (suspected winit); the workaround is closing to the tray. Details in the ledger.

## Pointers
- docs/ledgers/monitor-lock.md (full history of the fixes and user reports)
- Pin/auto rules: module doc in src/core.rs
